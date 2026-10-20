"""Reviewed image-backed exceptions for compatibility-only condition text.

These records never become canonical fields or synthetic source observations.
The complete correction document owns both historical canonical corrections and
the separately typed compatibility exceptions.
"""
from __future__ import annotations

from dataclasses import dataclass
import hashlib
import json
import os
from pathlib import Path
import re
from types import MappingProxyType
from typing import Any, Mapping

from .compatibility_selections import CompatibilitySourceSelection
from .errors import CardDataError, SourceSchemaError, ValidationError
from .models import ReviewedCorrection
from .serialization import canonical_json_text, content_digest, canonicalize

CORRECTIONS_PATH = 'data/card_corrections.json'
IMAGE_PREFIX = 'data/card_correction_evidence/'


@dataclass(frozen=True, slots=True)
class CompatibilityCorrection:
    schema_version: int
    correction_id: str
    correction_kind: str
    card_id: str
    field_path: str
    source_manifest_revision: str
    candidate_values: Mapping[str, str]
    candidate_set_digest: str
    base_observation_id: str
    base_value: str
    base_value_digest: str
    value: str
    value_digest: str
    evidence_observation_ids: tuple[str, ...]
    image_artifact_path: str
    image_sha256: str
    reason: str
    reviewer: str
    reviewed_at: str

    def __post_init__(self):
        try:
            CompatibilitySourceSelection(
                schema_version=self.schema_version, selection_id=self.correction_id,
                selection_kind='official_condition_projection', card_id=self.card_id,
                field_path=self.field_path, source_manifest_revision=self.source_manifest_revision,
                candidate_values=self.candidate_values, candidate_set_digest=self.candidate_set_digest,
                selected_observation_id=self.base_observation_id, selected_value=self.base_value,
                selected_value_digest=self.base_value_digest, reviewer=self.reviewer,
                reason=self.reason, reviewed_at=self.reviewed_at)
        except (CardDataError, TypeError, ValueError) as exc:
            raise ValidationError('invalid compatibility correction base binding') from exc
        if self.correction_kind != 'official_condition_image_correction':
            raise ValidationError('unsupported compatibility correction kind')
        if (not isinstance(self.value, str) or not self.value.strip()
                or self.value == self.base_value or content_digest(self.value) != self.value_digest):
            raise ValidationError('compatibility correction is empty, no-op, or has stale value digest')
        if (not isinstance(self.evidence_observation_ids, tuple)
                or tuple(sorted(self.candidate_values)) != tuple(sorted(self.evidence_observation_ids))):
            raise ValidationError('compatibility correction requires exact full observation evidence')
        if (not isinstance(self.image_sha256, str) or not re.fullmatch('[0-9a-f]{64}', self.image_sha256)
                or not isinstance(self.image_artifact_path, str)
                or self.image_artifact_path not in {
                    f'{IMAGE_PREFIX}{self.image_sha256}.{ext}' for ext in ('png', 'webp', 'jpg')}):
            raise ValidationError('compatibility correction image must be content-addressed and owned')
        object.__setattr__(self, 'candidate_values', MappingProxyType(dict(sorted(self.candidate_values.items()))))
        object.__setattr__(self, 'evidence_observation_ids', tuple(sorted(self.evidence_observation_ids)))


@dataclass(frozen=True, slots=True)
class CorrectionDocument:
    corrections: tuple[ReviewedCorrection, ...] = ()
    compatibility_corrections: tuple[CompatibilityCorrection, ...] = ()

    def __post_init__(self):
        from .corrections import validate_correction_set
        canonical = validate_correction_set(self.corrections)
        compatibility = tuple(sorted(self.compatibility_corrections, key=lambda x: (x.card_id, x.correction_id)))
        ids = [x.correction_id for x in (*canonical, *compatibility)]
        if len(ids) != len(set(ids)):
            raise ValidationError('correction IDs must be unique across the complete document')
        targets = [(x.card_id, x.field_path, x.source_manifest_revision) for x in compatibility]
        if len(targets) != len(set(targets)):
            raise ValidationError('overlapping compatibility corrections for one source revision')
        object.__setattr__(self, 'corrections', canonical)
        object.__setattr__(self, 'compatibility_corrections', compatibility)


def correction_document_payload(document: CorrectionDocument) -> dict[str, Any]:
    if not document.compatibility_corrections:
        return {'schema_version': 1, 'corrections': document.corrections}
    return {'schema_version': 2, 'corrections': document.corrections,
            'compatibility_corrections': document.compatibility_corrections}


def correction_document_from_payload(payload: Any) -> CorrectionDocument:
    from .corrections import _correction_from_dict
    if not isinstance(payload, Mapping) or type(payload.get('schema_version')) is not int:
        raise ValidationError('invalid correction document')
    version = payload['schema_version']
    expected = {'schema_version', 'corrections'} | ({'compatibility_corrections'} if version == 2 else set())
    if version not in (1, 2) or set(payload) != expected or not isinstance(payload.get('corrections'), list):
        raise ValidationError('unsupported correction document schema or fields')
    raw = payload.get('compatibility_corrections', [])
    if not isinstance(raw, list):
        raise ValidationError('compatibility corrections must be an array')
    rows = []
    try:
        for item in raw:
            item = dict(item)
            if not isinstance(item.get('evidence_observation_ids'), list):
                raise TypeError('evidence must be an array')
            item['evidence_observation_ids'] = tuple(item['evidence_observation_ids'])
            rows.append(CompatibilityCorrection(**item))
        return CorrectionDocument(
            tuple(_correction_from_dict(x, index=i) for i, x in enumerate(payload['corrections'])), tuple(rows))
    except (TypeError, ValueError, KeyError) as exc:
        raise ValidationError('invalid complete correction document entry') from exc


def _unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValidationError('duplicate key in correction document')
        result[key] = value
    return result


def correction_document_from_bytes(raw: bytes) -> CorrectionDocument:
    try:
        return correction_document_from_payload(json.loads(raw.decode('utf-8'), object_pairs_hook=_unique_object))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ValidationError('invalid correction document JSON') from exc


def load_correction_document(path: Path) -> CorrectionDocument:
    return correction_document_from_bytes(path.read_bytes()) if path.exists() else CorrectionDocument()


def require_document_extension(existing: CorrectionDocument, proposed: CorrectionDocument) -> None:
    old = {x.correction_id: x for x in (*existing.corrections, *existing.compatibility_corrections)}
    new = {x.correction_id: x for x in (*proposed.corrections, *proposed.compatibility_corrections)}
    if any(new.get(key) != value for key, value in old.items()):
        raise ValidationError('complete correction document must preserve all existing entries')


def write_correction_document(path: Path, document: CorrectionDocument) -> None:
    require_document_extension(load_correction_document(path), document)
    temporary = path.with_name(f'.{path.name}.{os.getpid()}.tmp')
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        temporary.write_text(canonical_json_text(correction_document_payload(document)), encoding='utf-8')
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)


def validate_image_artifacts(document: CorrectionDocument, artifacts: Mapping[str, bytes]) -> None:
    for row in document.compatibility_corrections:
        raw = artifacts.get(row.image_artifact_path)
        if not isinstance(raw, bytes) or hashlib.sha256(raw).hexdigest() != row.image_sha256:
            raise ValidationError('reviewed correction image is missing or has changed',
                                  card_id=row.card_id, path=row.image_artifact_path)


def validate_correction_artifacts(artifacts: Mapping[str, bytes]) -> CorrectionDocument:
    raw = artifacts.get(CORRECTIONS_PATH)
    document = correction_document_from_bytes(raw) if raw is not None else CorrectionDocument()
    validate_image_artifacts(document, artifacts)
    return document


def load_document_images(document: CorrectionDocument, root: Path) -> dict[str, bytes]:
    """Acquire staged images by owned logical path; callers seal returned bytes."""
    images = {}
    for row in document.compatibility_corrections:
        path = (root / row.image_artifact_path).resolve()
        if not path.is_relative_to(root.resolve()) or not path.is_file():
            raise ValidationError('correction image artifact is unavailable', path=path)
        images[row.image_artifact_path] = path.read_bytes()
    validate_image_artifacts(document, images)
    return images


def candidate_document(corrections, document, image_artifacts):
    if document is not None and corrections:
        raise ValidationError('provide complete correction document or legacy rows, not both')
    document = document if document is not None else CorrectionDocument(tuple(corrections))
    images = {r.image_artifact_path: image_artifacts[r.image_artifact_path]
              for r in document.compatibility_corrections if r.image_artifact_path in image_artifacts}
    validate_image_artifacts(document, images)
    return document, images


def correction_review_rows(document: CorrectionDocument, revision: str):
    return [{**canonicalize(row), 'status': 'applied' if row.source_manifest_revision == revision else 'historical'}
            for row in document.compatibility_corrections]


def apply_image_correction(card, observations, correction: CompatibilityCorrection, *,
                           source_manifest_revision: str, image_artifacts: Mapping[str, bytes]):
    from .compatibility_conditions import official_condition_projection
    validate_image_artifacts(CorrectionDocument(compatibility_corrections=(correction,)), image_artifacts)
    if correction.card_id != card.card_id or correction.source_manifest_revision != source_manifest_revision:
        raise ValidationError('compatibility correction has stale source or foreign card binding')
    try:
        value, evidence = official_condition_projection(card, observations)
        values = {oid: value for oid in evidence}
    except SourceSchemaError as exc:
        values = exc.details.get('candidate_conditions')
        if not isinstance(values, Mapping):
            raise ValidationError('compatibility correction source evidence is invalid') from exc
    if dict(values) != dict(correction.candidate_values) or content_digest(values) != correction.candidate_set_digest:
        raise ValidationError('compatibility correction candidate evidence/value set changed')
    return correction.value, correction.evidence_observation_ids
