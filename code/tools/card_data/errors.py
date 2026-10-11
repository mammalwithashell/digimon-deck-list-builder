"""Stable diagnostics and typed failures for card-data tooling.

Codes and serialized field names in this module are public tooling contracts.
Callers may add context in ``details``, but should not reinterpret one failure
class as another.  In particular, only ``NetworkError`` represents an offline
transport failure; receiving an HTTP response or failing to parse it is not an
offline condition.
"""

from __future__ import annotations

import json
import os
from collections import Counter
from dataclasses import dataclass, field
from enum import StrEnum
from typing import Any, Mapping


class DiagnosticCode(StrEnum):
    """Stable machine-readable identifiers for the foundation taxonomy."""

    NETWORK_ERROR = "NETWORK_ERROR"
    HTTP_ERROR = "HTTP_ERROR"
    PARSE_ERROR = "PARSE_ERROR"
    SOURCE_SCHEMA_ERROR = "SOURCE_SCHEMA_ERROR"
    UNSUPPORTED_LAYOUT = "UNSUPPORTED_LAYOUT"
    COMPLETENESS_ERROR = "COMPLETENESS_ERROR"
    ADJUDICATION_ERROR = "ADJUDICATION_ERROR"
    VALIDATION_ERROR = "VALIDATION_ERROR"
    WRITE_ERROR = "WRITE_ERROR"


class DiagnosticCategory(StrEnum):
    """Broad failure groups used for aggregation and policy decisions."""

    NETWORK = "network"
    HTTP = "http"
    PARSE = "parse"
    SOURCE_SCHEMA = "source_schema"
    UNSUPPORTED_LAYOUT = "unsupported_layout"
    COMPLETENESS = "completeness"
    ADJUDICATION = "adjudication"
    VALIDATION = "validation"
    WRITE = "write"


class DiagnosticSeverity(StrEnum):
    ERROR = "error"
    WARNING = "warning"
    INFO = "info"


@dataclass(frozen=True, slots=True)
class Diagnostic:
    """One serializable card-data diagnostic with optional origin context."""

    code: DiagnosticCode
    category: DiagnosticCategory
    severity: DiagnosticSeverity
    message: str
    card_id: str | None = None
    source: str | None = None
    path: str | None = None
    field_path: str | None = None
    details: Mapping[str, Any] = field(default_factory=dict)

    def __post_init__(self) -> None:
        if not self.message:
            raise ValueError("diagnostic message must not be empty")
        if not isinstance(self.code, DiagnosticCode):
            raise TypeError("diagnostic code must be a DiagnosticCode")
        if not isinstance(self.category, DiagnosticCategory):
            raise TypeError("diagnostic category must be a DiagnosticCategory")
        if not isinstance(self.severity, DiagnosticSeverity):
            raise TypeError("diagnostic severity must be a DiagnosticSeverity")

    def to_dict(self) -> dict[str, Any]:
        """Return the stable machine-readable diagnostic shape."""

        return {
            "code": self.code.value,
            "category": self.category.value,
            "severity": self.severity.value,
            "message": self.message,
            "card_id": self.card_id,
            "source": self.source,
            "path": self.path,
            "field_path": self.field_path,
            "details": dict(self.details),
        }


@dataclass(frozen=True, slots=True)
class DiagnosticReport:
    """An aggregate that retains every applicable diagnostic."""

    diagnostics: tuple[Diagnostic, ...] = ()

    @property
    def has_errors(self) -> bool:
        return any(item.severity is DiagnosticSeverity.ERROR for item in self.diagnostics)

    def to_dict(self) -> dict[str, Any]:
        severity_counts = Counter(item.severity.value for item in self.diagnostics)
        category_counts = Counter(item.category.value for item in self.diagnostics)
        return {
            "summary": {
                "total": len(self.diagnostics),
                "by_severity": {
                    severity.value: severity_counts[severity.value]
                    for severity in DiagnosticSeverity
                },
                "by_category": dict(sorted(category_counts.items())),
            },
            "diagnostics": [item.to_dict() for item in self.diagnostics],
        }

    def to_json(self, *, indent: int | None = 2) -> str:
        return json.dumps(
            self.to_dict(),
            ensure_ascii=False,
            indent=indent,
            sort_keys=True,
        )


class CardDataError(Exception):
    """Base class for failures that cross a card-data tool boundary."""

    code: DiagnosticCode
    category: DiagnosticCategory
    _is_offline = False

    def __init__(
        self,
        message: str,
        *,
        card_id: str | None = None,
        source: str | None = None,
        path: str | os.PathLike[str] | None = None,
        field_path: str | None = None,
        severity: DiagnosticSeverity = DiagnosticSeverity.ERROR,
        details: Mapping[str, Any] | None = None,
    ) -> None:
        if not message:
            raise ValueError("card-data error message must not be empty")
        super().__init__(message)
        self.message = message
        self.card_id = card_id
        self.source = source
        self.path = os.fspath(path) if path is not None else None
        self.field_path = field_path
        self.severity = severity
        self.details = dict(details or {})

    @property
    def is_offline(self) -> bool:
        """Whether the failure occurred before any source response arrived."""

        return self._is_offline

    @property
    def diagnostic(self) -> Diagnostic:
        return Diagnostic(
            code=self.code,
            category=self.category,
            severity=self.severity,
            message=self.message,
            card_id=self.card_id,
            source=self.source,
            path=self.path,
            field_path=self.field_path,
            details=self.details,
        )


class NetworkError(CardDataError):
    code = DiagnosticCode.NETWORK_ERROR
    category = DiagnosticCategory.NETWORK
    _is_offline = True


class HttpError(CardDataError):
    code = DiagnosticCode.HTTP_ERROR
    category = DiagnosticCategory.HTTP

    def __init__(
        self,
        message: str,
        *,
        status_code: int | None = None,
        source_url: str | None = None,
        details: Mapping[str, Any] | None = None,
        **context: Any,
    ) -> None:
        merged_details = dict(details or {})
        if source_url is not None:
            merged_details["source_url"] = source_url
        if status_code is not None:
            merged_details["status_code"] = status_code
        super().__init__(message, details=merged_details, **context)
        self.status_code = status_code
        self.source_url = source_url


class ParseError(CardDataError):
    code = DiagnosticCode.PARSE_ERROR
    category = DiagnosticCategory.PARSE


class SourceSchemaError(CardDataError):
    code = DiagnosticCode.SOURCE_SCHEMA_ERROR
    category = DiagnosticCategory.SOURCE_SCHEMA


class UnsupportedLayoutError(CardDataError):
    code = DiagnosticCode.UNSUPPORTED_LAYOUT
    category = DiagnosticCategory.UNSUPPORTED_LAYOUT


class CompletenessError(CardDataError):
    code = DiagnosticCode.COMPLETENESS_ERROR
    category = DiagnosticCategory.COMPLETENESS


class AdjudicationError(CardDataError):
    code = DiagnosticCode.ADJUDICATION_ERROR
    category = DiagnosticCategory.ADJUDICATION


class ValidationError(CardDataError):
    code = DiagnosticCode.VALIDATION_ERROR
    category = DiagnosticCategory.VALIDATION


class WriteError(CardDataError):
    code = DiagnosticCode.WRITE_ERROR
    category = DiagnosticCategory.WRITE
