"""Canonical card-data acquisition, normalization, and generation tooling.

This package is the shared library boundary for card-data tools.  Legacy
scripts will migrate to it incrementally; importing the package must therefore
remain side-effect free and must never perform network or filesystem writes.
"""

from .errors import (
    AdjudicationError,
    CardDataError,
    CompletenessError,
    Diagnostic,
    DiagnosticCategory,
    DiagnosticCode,
    DiagnosticReport,
    DiagnosticSeverity,
    HttpError,
    NetworkError,
    ParseError,
    SourceSchemaError,
    UnsupportedLayoutError,
    ValidationError,
    WriteError,
)

__version__ = "0.1.0"

__all__ = [
    "AdjudicationError",
    "CardDataError",
    "CompletenessError",
    "Diagnostic",
    "DiagnosticCategory",
    "DiagnosticCode",
    "DiagnosticReport",
    "DiagnosticSeverity",
    "HttpError",
    "NetworkError",
    "ParseError",
    "SourceSchemaError",
    "UnsupportedLayoutError",
    "ValidationError",
    "WriteError",
    "__version__",
]
