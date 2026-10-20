"""Explicit offline reader for reviewed v2 artifacts, never an admission API.

The expected byte digest must come from the caller's external review. Internal
reconstruction detects inconsistent contents; it cannot authenticate a source
or turn a caller-computed digest into approval. Existing live readers stay v1.
"""
from dataclasses import dataclass
import hashlib
import json
import re
from types import MappingProxyType

from .source_book_v2 import validate_wire_book_v2


def _pairs(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f'duplicate JSON key: {key}')
        result[key] = value
    return result


def _constant(value):
    raise ValueError(f'nonfinite JSON constant: {value}')


def _decode(raw):
    return json.loads(raw.decode('utf-8'), object_pairs_hook=_pairs,
                      parse_constant=_constant)


@dataclass(frozen=True, slots=True)
class ReviewBindingV2:
    semantic_version: int
    source_generation_sha256: str
    semantic_sha256: str


@dataclass(frozen=True, slots=True)
class ReviewBookV2:
    """Immutable offline evidence handle, deliberately unrelated to v1 Book."""
    _raw: bytes
    _counts: object
    _bindings: object
    source_generation_sha256: str

    @property
    def card_count(self):
        return len(self._counts)

    @property
    def is_recordable(self):
        return False

    def clause_ids(self):
        return tuple(self._bindings)

    def card_clause_count(self, card_id):
        try:
            return self._counts[card_id]
        except KeyError as exc:
            raise ValueError(f'card absent from source inventory: {card_id}') from exc

    def binding(self, clause_id):
        try:
            return self._bindings[clause_id]
        except KeyError as exc:
            raise ValueError(f'clause absent from review book: {clause_id}') from exc

    def document(self):
        """Return a fresh evidence copy; mutations cannot change this handle."""
        return _decode(self._raw)


def load_review_v2(raw: bytes, *, expected_sha256: str) -> ReviewBookV2:
    """Load exact externally pinned bytes, reconstructing all v2 wire contents."""
    if type(raw) is not bytes:
        raise ValueError('review book must be exact bytes')
    if (type(expected_sha256) is not str or
            re.fullmatch(r'[0-9a-f]{64}', expected_sha256) is None or
            hashlib.sha256(raw).hexdigest() != expected_sha256):
        raise ValueError('review book byte pin mismatch')
    try:
        doc = _decode(raw)
        if type(doc) is not dict:
            raise ValueError('review book must be an object')
        if type(doc.get('diagnostic_evidence')) is not dict:
            raise ValueError('review diagnostic must be an object')
        validate_wire_book_v2(doc)
    except (KeyError, TypeError, IndexError, AttributeError, UnicodeError, OverflowError) as exc:
        raise ValueError('malformed review-v2 book') from exc
    generation = doc['source_generation_sha256']
    counts = MappingProxyType({cid: row['clause_count']
                               for cid, row in doc['cards'].items()})
    bindings = MappingProxyType({row['id']: ReviewBindingV2(
        2, generation, row['semantic_sha256']) for row in doc['clauses']})
    return ReviewBookV2(raw, counts, bindings, generation)
