"""Secondary digimoncard.io discovery and enrichment adapter.

This adapter preserves the upstream record and identifies explicit versus
heuristically completed evolution routes.  Heuristic candidates are diagnostic
only: they are marked derived/non-authoritative and never enter the adapter's
``explicit_evolution_routes`` values.
"""

from __future__ import annotations

import dataclasses
import json
import re
import time
import urllib.request
from dataclasses import dataclass
from datetime import datetime, timezone
from typing import Any, Callable, Mapping
from urllib.parse import urlencode

from ..cache import (
    ContentAddressedResponseCache,
    FetchedPayload,
    OpenUrl,
    RetryPolicy,
    Sleep,
    fetch_with_retry,
)
from ..errors import (
    CompletenessError,
    ParseError,
    SourceSchemaError,
    UnsupportedLayoutError,
    ValidationError,
)
from ..models import SCHEMA_VERSION, SourceName, SourceObservation
from ..serialization import observation_digest, sha256_hex


API_BASE_URL = "https://digimoncard.io/api-public/search"
FULL_POOL_SCOPE = "full-pool:digimon-card-game"
# https://digimoncard.io/api-documentation: search cap and public rate limit.
CATALOG_BATCH_SIZE = 100
SEARCH_RESPONSE_LIMIT = 1000
CATALOG_REQUEST_INTERVAL_SECONDS = 0.75
PARSER_VERSION = "digimoncard-io-json-v1"
SOURCE_SCHEMA_VERSION = "digimoncard-io-public-api-v1"
CARD_ID_PATTERN = re.compile(r"^[A-Z]+\d*-\d+[A-Z]*$")
INHERITED_PREFIX = re.compile(r"^\s*Inherited Effect\s*", re.IGNORECASE)
COLOR_ORDER = ("Red", "Blue", "Yellow", "Green", "White", "Black", "Purple")
KNOWN_COLORS = frozenset(COLOR_ORDER)
KNOWN_CARD_TYPES = frozenset({"Digimon", "Tamer", "Option", "Digi-Egg", "Dual"})


Clock = Callable[[], str]


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


@dataclass(frozen=True, slots=True)
class ParsedDigimonCardIOScope:
    """Validated secondary rows and distinct gameplay IDs for one scope."""

    scope: str
    source_url: str
    raw_payload: bytes
    raw_payload_digest: str
    records: tuple[Mapping[str, Any], ...]
    card_ids: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class DigimonCardIOAcquisition:
    """A fetched secondary scope plus immutable source observations."""

    parsed: ParsedDigimonCardIOScope
    observations: tuple[SourceObservation, ...]
    last_checked: str
    fetch_attempts: int


@dataclass(frozen=True, slots=True)
class DigimonCardIOCatalogAcquisition:
    """Coverage-checked catalog with original response provenance per batch."""

    discovery_url: str
    discovery_payload_digest: str
    card_ids: tuple[str, ...]
    acquisitions: tuple[DigimonCardIOAcquisition, ...]



def _card_id(record: Mapping[str, Any], *, row_number: int) -> str:
    value = record.get("id")
    if not isinstance(value, str) or not CARD_ID_PATTERN.fullmatch(value.upper()):
        raise SourceSchemaError(
            "digimoncard.io row is missing a valid gameplay card ID",
            source=SourceName.DIGIMONCARD_IO.value,
            field_path="id",
            details={"row_number": row_number, "value": value},
        )
    return value.upper()


def parse_scope_payload(
    payload: bytes,
    *,
    scope: str,
    source_url: str,
) -> ParsedDigimonCardIOScope:
    """Decode and schema-check one digimoncard.io public-API response."""

    try:
        text = payload.decode("utf-8")
        decoded = json.loads(text)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ParseError(
            "digimoncard.io response is not valid UTF-8 JSON",
            source=source_url,
            details={"error": str(exc)},
        ) from exc
    if not isinstance(decoded, list):
        raise SourceSchemaError(
            "digimoncard.io response root must be an array",
            source=source_url,
            details={"actual_type": type(decoded).__name__},
        )

    raw_scope = scope.strip()
    full_pool = raw_scope.casefold() == FULL_POOL_SCOPE.casefold()
    normalized_scope = FULL_POOL_SCOPE if full_pool else raw_scope.upper()
    if not full_pool and (
        not normalized_scope or not re.fullmatch(r"[A-Z]+\d*", normalized_scope)
    ):
        raise ValueError(f"invalid card scope prefix: {scope!r}")
    expected_prefix = None if full_pool else f"{normalized_scope}-"

    records: list[Mapping[str, Any]] = []
    distinct_ids: set[str] = set()
    distinct_records: set[str] = set()
    for row_number, row in enumerate(decoded):
        if not isinstance(row, Mapping):
            raise SourceSchemaError(
                "digimoncard.io response row must be an object",
                source=source_url,
                details={"row_number": row_number, "actual_type": type(row).__name__},
            )
        card_id = _card_id(row, row_number=row_number)
        # Prefix searches are over-broad (BT1 also returns BT10, etc.).  Retain
        # only exact scope members, while preserving duplicate printing rows.
        if expected_prefix is not None and not card_id.startswith(expected_prefix):
            continue
        copied = dict(row)
        copied["id"] = card_id
        record_digest = _stable_record_suffix(copied)
        if record_digest in distinct_records:
            continue
        distinct_records.add(record_digest)
        records.append(copied)
        distinct_ids.add(card_id)

    records.sort(key=lambda row: (str(row["id"]), _stable_record_suffix(row)))
    return ParsedDigimonCardIOScope(
        scope=normalized_scope,
        source_url=source_url,
        raw_payload=payload,
        raw_payload_digest=sha256_hex(payload),
        records=tuple(records),
        card_ids=tuple(sorted(distinct_ids)),
    )


def _stable_record_suffix(record: Mapping[str, Any]) -> str:
    return sha256_hex(
        json.dumps(record, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode(
            "utf-8"
        )
    )


def _explicit_evolution_routes(record: Mapping[str, Any]) -> list[dict[str, Any]]:
    routes: list[dict[str, Any]] = []
    for ordinal in (1, 2, 3):
        suffix = "" if ordinal == 1 else f"_{ordinal}"
        cost = record.get(f"evolution_cost{suffix}")
        color = record.get(f"evolution_color{suffix}")
        level = record.get(f"evolution_level{suffix}")
        if cost is None and color is None and level is None:
            continue
        if cost is not None and color not in (None, "") and level is not None:
            routes.append(
                {
                    "ordinal": ordinal,
                    "color": color,
                    "level": level,
                    "cost": cost,
                    "derived": False,
                    "authority": "secondary",
                }
            )
    return routes


def _optional_int(value: Any, *, card_id: str, field_path: str) -> int | None:
    if value in (None, "", "-"):
        return None
    if isinstance(value, bool):
        raise SourceSchemaError(
            "digimoncard.io numeric field cannot be boolean",
            source=SourceName.DIGIMONCARD_IO.value,
            card_id=card_id,
            field_path=field_path,
        )
    if isinstance(value, int):
        parsed = value
    elif isinstance(value, str) and value.strip().isdigit():
        parsed = int(value.strip())
    else:
        raise SourceSchemaError(
            "digimoncard.io numeric field has an unsupported value",
            source=SourceName.DIGIMONCARD_IO.value,
            card_id=card_id,
            field_path=field_path,
            details={"value": value},
        )
    if parsed < 0:
        raise SourceSchemaError(
            "digimoncard.io numeric field cannot be negative",
            source=SourceName.DIGIMONCARD_IO.value,
            card_id=card_id,
            field_path=field_path,
        )
    return parsed


def _colors(
    record: Mapping[str, Any],
    keys: tuple[str, ...],
    *,
    card_id: str,
    required: bool = True,
) -> list[str]:
    colors: list[str] = []
    for key in keys:
        value = record.get(key)
        if value in (None, ""):
            continue
        if not isinstance(value, str) or value not in KNOWN_COLORS:
            raise SourceSchemaError(
                "digimoncard.io color field has an unsupported value",
                source=SourceName.DIGIMONCARD_IO.value,
                card_id=card_id,
                field_path=key,
                details={"value": value},
            )
        if value not in colors:
            colors.append(value)
    if required and not colors:
        raise SourceSchemaError(
            "digimoncard.io card record has no explicit color",
            source=SourceName.DIGIMONCARD_IO.value,
            card_id=card_id,
            field_path=keys[0],
        )
    return colors


def _traits(record: Mapping[str, Any]) -> list[str]:
    return [
        str(record[key]).strip()
        for key in ("digi_type", "digi_type2", "digi_type3", "digi_type4")
        if record.get(key) not in (None, "") and str(record[key]).strip()
    ]


def _canonical_evolution_routes(
    record: Mapping[str, Any], *, card_id: str
) -> list[dict[str, Any]]:
    routes: list[dict[str, Any]] = []
    for route in _explicit_evolution_routes(record):
        color = route["color"]
        if color == "All":
            colors = list(COLOR_ORDER)
        elif color in KNOWN_COLORS:
            colors = [color]
        else:
            raise SourceSchemaError(
                "digimoncard.io evolution route has an unsupported color",
                source=SourceName.DIGIMONCARD_IO.value,
                card_id=card_id,
                field_path=f"evolution_color_{route['ordinal']}",
                details={"value": color},
            )
        from_level = _optional_int(
            route["level"],
            card_id=card_id,
            field_path=f"evolution_level_{route['ordinal']}",
        )
        cost = _optional_int(
            route["cost"],
            card_id=card_id,
            field_path=f"evolution_cost_{route['ordinal']}",
        )
        if from_level is None or cost is None:
            raise SourceSchemaError(
                "digimoncard.io explicit evolution route is incomplete",
                source=SourceName.DIGIMONCARD_IO.value,
                card_id=card_id,
                field_path="evolution_routes",
                details={"ordinal": route["ordinal"]},
            )
        routes.append({"colors": colors, "from_level": from_level, "cost": cost})
    return routes


def _canonical_projection(
    record: Mapping[str, Any],
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    card_id = _card_id(record, row_number=0)
    card_type = record.get("type")
    if card_type not in KNOWN_CARD_TYPES:
        raise UnsupportedLayoutError(
            "digimoncard.io card type is not represented by the canonical schema",
            source=SourceName.DIGIMONCARD_IO.value,
            card_id=card_id,
            field_path="type",
            details={"value": card_type},
        )
    primary_colors = _colors(
        record,
        ("color", "color2", "color3"),
        card_id=card_id,
    )
    play_cost = _optional_int(record.get("play_cost"), card_id=card_id, field_path="play_cost")
    level = _optional_int(record.get("level"), card_id=card_id, field_path="level")
    dp = _optional_int(record.get("dp"), card_id=card_id, field_path="dp")
    has_evolution_route = any(
        record.get(f"evolution_cost{suffix}") is not None
        for suffix in ("", "_2", "_3")
    )
    if card_type in {"Digimon", "Dual"} and level is None and has_evolution_route:
        raise SourceSchemaError(
            "level-less digimoncard.io card cannot declare a level evolution route",
            source=SourceName.DIGIMONCARD_IO.value,
            card_id=card_id,
            field_path="faces.0.level",
        )
    if not isinstance(record.get("name"), str) or not str(record["name"]).strip():
        raise SourceSchemaError(
            "digimoncard.io card record is missing name",
            source=SourceName.DIGIMONCARD_IO.value,
            card_id=card_id,
            field_path="name",
        )
    main_effect = str(record.get("main_effect") or "")
    inherited_effect = str(record.get("source_effect") or "")
    security_effect = str(record.get("security_effect") or "")
    if card_type != "Dual" and not inherited_effect.strip():
        match = INHERITED_PREFIX.match(main_effect)
        if match:
            inherited_effect = main_effect[match.end() :].strip()
            main_effect = ""
    routes = _canonical_evolution_routes(record, card_id=card_id)
    canonical_kind = "Digimon" if card_type == "Dual" else card_type
    fields = {
        "card_name_eng": str(record.get("name") or ""),
        "card_kind": canonical_kind,
        "card_colors": primary_colors,
        "dp": dp,
        "level": level,
        "type_eng": _traits(record),
        "form_eng": [str(record["form"]).strip()] if record.get("form") else [],
        "attribute_eng": (
            [str(record["attribute"]).strip()] if record.get("attribute") else []
        ),
        "effect_description_eng": main_effect,
        "inherited_effect_description_eng": inherited_effect,
        "security_effect_description_eng": security_effect,
        "evo_costs": routes,
    }
    if play_cost is not None:
        fields["play_cost"] = play_cost
    if dp is None:
        fields.pop("dp", None)

    if card_type == "Dual":
        option_colors = _colors(
            record,
            ("option_color", "option_color1", "option_color2", "option_color3"),
            card_id=card_id,
            required=False,
        )
        faces = [] if not option_colors else [
            {
                "face_id": "digimon",
                "kind": "digimon",
                "colors": primary_colors,
                "play_cost": None,
                "level": level,
                "dp": dp,
                "effect_text": main_effect,
                "inherited_text": "",
                "security_text": "",
            },
            {
                "face_id": "option",
                "kind": "option",
                "colors": option_colors,
                "use_cost": play_cost or 0,
                "effect_text": inherited_effect,
                "inherited_text": "",
                "security_text": security_effect,
            },
        ]
    else:
        faces = [
            {
                "face_id": "primary",
                "kind": str(card_type).lower().replace("-", "_"),
                "colors": primary_colors,
                "play_cost": play_cost,
                "level": level,
                "dp": dp,
                "effect_text": main_effect,
                "inherited_text": inherited_effect,
                "security_text": security_effect,
            }
        ]
    return fields, faces


def _derived_evolution_candidates(record: Mapping[str, Any]) -> list[dict[str, Any]]:
    """Return likely route completions as diagnostics, never selected values."""

    candidates: list[dict[str, Any]] = []
    card_level = record.get("level")
    for ordinal in (1, 2, 3):
        suffix = "" if ordinal == 1 else f"_{ordinal}"
        cost = record.get(f"evolution_cost{suffix}")
        explicit_color = record.get(f"evolution_color{suffix}")
        explicit_level = record.get(f"evolution_level{suffix}")
        if cost is None or (explicit_color not in (None, "") and explicit_level is not None):
            continue

        candidate_color = explicit_color
        if candidate_color in (None, ""):
            color_key = "color" if ordinal == 1 else f"color{ordinal}"
            candidate_color = record.get(color_key)
        candidate_level = explicit_level
        if candidate_level is None and isinstance(card_level, int) and card_level >= 3:
            candidate_level = card_level - 1
        if candidate_color in (None, "") or candidate_level is None:
            continue
        candidates.append(
            {
                "field_path": "evolution_routes",
                "candidate": {
                    "ordinal": ordinal,
                    "color": candidate_color,
                    "level": candidate_level,
                    "cost": cost,
                },
                "derived": True,
                "authoritative": False,
                "reason": "digimoncard.io omitted explicit evolution route components",
            }
        )
    return candidates


def extract_enrichment(record: Mapping[str, Any]) -> dict[str, Any]:
    """Preserve source data and separate non-authoritative derived candidates."""

    card_id = _card_id(record, row_number=0)
    source_record = dict(record)
    source_record["id"] = card_id
    canonical_fields, faces = _canonical_projection(source_record)
    structural_diagnostics = []
    if source_record.get("type") == "Dual" and not faces:
        structural_diagnostics.append(
            {
                "field_path": "faces.1.colors",
                "derived": False,
                "authoritative": False,
                "reason": "digimoncard.io omitted the explicit Option-face color",
            }
        )
    return {
        "card_id": card_id,
        "source_record": source_record,
        "canonical_fields": canonical_fields,
        "faces": faces,
        "explicit_evolution_routes": _explicit_evolution_routes(source_record),
        "derived_diagnostics": _derived_evolution_candidates(source_record),
        "structural_diagnostics": structural_diagnostics,
    }


def make_source_observation(
    *,
    raw_payload: bytes,
    extracted: Mapping[str, Any],
    source_record_key: str,
    source_url: str,
    first_acquired_at: str,
    parser_version: str = PARSER_VERSION,
    source_schema_version: str = SOURCE_SCHEMA_VERSION,
) -> SourceObservation:
    """Create an immutable observation with its content-derived identity."""

    draft = SourceObservation(
        schema_version=SCHEMA_VERSION,
        observation_id="0" * 64,
        source=SourceName.DIGIMONCARD_IO,
        source_record_key=source_record_key,
        source_url=source_url,
        raw_payload_digest=sha256_hex(raw_payload),
        parser_version=parser_version,
        source_schema_version=source_schema_version,
        first_acquired_at=first_acquired_at,
        extracted=extracted,
    )
    return dataclasses.replace(draft, observation_id=observation_digest(draft))


def observations_from_scope(
    parsed: ParsedDigimonCardIOScope,
    *,
    first_acquired_at: str,
    parser_version: str = PARSER_VERSION,
    source_schema_version: str = SOURCE_SCHEMA_VERSION,
    existing_observations: Mapping[str, SourceObservation] | None = None,
) -> tuple[SourceObservation, ...]:
    observations: list[SourceObservation] = []
    duplicate_counts: dict[str, int] = {}
    for record in parsed.records:
        card_id = str(record["id"])
        ordinal = duplicate_counts.get(card_id, 0)
        duplicate_counts[card_id] = ordinal + 1
        key = card_id if ordinal == 0 else f"{card_id}#{ordinal + 1}"
        candidate = make_source_observation(
            raw_payload=parsed.raw_payload,
            extracted=extract_enrichment(record),
            source_record_key=key,
            source_url=parsed.source_url,
            first_acquired_at=first_acquired_at,
            parser_version=parser_version,
            source_schema_version=source_schema_version,
        )
        existing = (existing_observations or {}).get(candidate.observation_id)
        if existing is not None:
            if observation_digest(existing) != candidate.observation_id:
                raise ValidationError(
                    "existing source observation does not match its content identity",
                    source=SourceName.DIGIMONCARD_IO.value,
                    details={"observation_id": candidate.observation_id},
                )
            observations.append(existing)
        else:
            observations.append(candidate)
    return tuple(observations)


class DigimonCardIOAdapter:
    """Acquire exact-prefix discovery/enrichment observations from the API."""

    def __init__(
        self,
        *,
        cache: ContentAddressedResponseCache | None = None,
        retry_policy: RetryPolicy = RetryPolicy(),
        timeout: float = 30.0,
        opener: OpenUrl | None = None,
        sleep: Sleep | None = None,
        clock: Clock = utc_now,
        parser_version: str = PARSER_VERSION,
        source_schema_version: str = SOURCE_SCHEMA_VERSION,
        existing_observations: Mapping[str, SourceObservation] | None = None,
    ) -> None:
        self.cache = cache
        self.retry_policy = retry_policy
        self.timeout = timeout
        self.opener = opener
        self.sleep = sleep
        self.clock = clock
        self.parser_version = parser_version
        self.source_schema_version = source_schema_version
        self.existing_observations = existing_observations

    def build_url(self, scope: str) -> str:
        return f"{API_BASE_URL}?{urlencode({'card': scope.strip().upper()})}"

    def build_catalog_url(self) -> str:
        """Discover complete identities; search responses are capped at 1,000."""
        return "https://digimoncard.io/api-public/getAllCards?" + urlencode({
            "series": "Digimon Card Game", "sort": "card_number", "sortdirection": "asc",
        })

    def _fetch(self, source_url: str, *, throttled: bool = False) -> FetchedPayload:
        options: dict[str, Any] = {
            "cache": self.cache, "policy": self.retry_policy, "timeout": self.timeout,
        }
        opener = self.opener or urllib.request.urlopen
        pause = self.sleep or time.sleep
        if throttled:
            def spaced_opener(request, **kwargs):
                # Every attempt, including retries, is below 15 requests/10s.
                pause(CATALOG_REQUEST_INTERVAL_SECONDS)
                return opener(request, **kwargs)
            options["opener"] = spaced_opener
        elif self.opener is not None:
            options["opener"] = self.opener
        if self.sleep is not None:
            options["sleep"] = self.sleep
        return fetch_with_retry(source_url, **options)

    def _acquire(self, *, source_url: str, scope: str,
                 throttled: bool = False) -> DigimonCardIOAcquisition:
        fetched = self._fetch(source_url, throttled=throttled)
        checked_at = self.clock()
        parsed = parse_scope_payload(
            fetched.body,
            scope=scope,
            source_url=source_url,
        )
        observations = observations_from_scope(
            parsed,
            first_acquired_at=checked_at,
            parser_version=self.parser_version,
            source_schema_version=self.source_schema_version,
            existing_observations=self.existing_observations,
        )
        return DigimonCardIOAcquisition(
            parsed=parsed,
            observations=observations,
            last_checked=checked_at,
            fetch_attempts=fetched.attempts,
        )

    def acquire_scope(self, scope: str) -> DigimonCardIOAcquisition:
        return self._acquire(
            source_url=self.build_url(scope),
            scope=scope,
        )

    def acquire_catalog(self) -> DigimonCardIOCatalogAcquisition:
        """Discover every identity, then acquire bounded full-record responses.

        Never concatenate response bodies into synthetic raw provenance. A
        saturated batch is split and re-fetched; a saturated singleton refuses
        completeness because this public API has no documented pagination.
        """
        discovery_url = self.build_catalog_url()
        fetched = self._fetch(discovery_url, throttled=True)
        try:
            discovered = json.loads(fetched.body.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            raise ParseError("secondary catalog discovery is not UTF-8 JSON") from exc
        if not isinstance(discovered, list) or not discovered:
            raise CompletenessError("secondary catalog discovery must be a nonempty array")
        ids: list[str] = []
        for index, record in enumerate(discovered):
            if not isinstance(record, Mapping):
                raise SourceSchemaError("secondary catalog discovery row is not an object")
            ids.append(_card_id({"id": record.get("cardnumber")}, row_number=index))
        if len(ids) != len(set(ids)):
            raise CompletenessError("secondary catalog discovery contains duplicate identities")
        ids.sort()
        batches: list[DigimonCardIOAcquisition] = []

        def acquire_batch(requested: list[str]) -> None:
            url = f"{API_BASE_URL}?" + urlencode({
                "series": "Digimon Card Game", "card": ",".join(requested),
                "sort": "code", "sortdirection": "asc", "limit": SEARCH_RESPONSE_LIMIT,
            })
            batch = self._acquire(source_url=url, scope=FULL_POOL_SCOPE, throttled=True)
            # Count raw rows, before parser deduplication: even duplicate rows
            # can occupy the response cap and conceal additional printings.
            raw_count = len(json.loads(batch.parsed.raw_payload.decode("utf-8")))
            actual = set(batch.parsed.card_ids)
            if actual - set(requested):
                raise CompletenessError("secondary catalog batch contains unexpected identities",
                                        details={"unexpected_ids": sorted(actual - set(requested))})
            if raw_count >= SEARCH_RESPONSE_LIMIT:
                if len(requested) == 1:
                    raise CompletenessError("secondary catalog singleton response is saturated")
                midpoint = len(requested) // 2
                acquire_batch(requested[:midpoint])
                acquire_batch(requested[midpoint:])
                return
            if actual != set(requested):
                raise CompletenessError("secondary catalog batch is missing discovered identities",
                                        details={"missing_ids": sorted(set(requested) - actual)})
            batches.append(batch)

        for offset in range(0, len(ids), CATALOG_BATCH_SIZE):
            acquire_batch(ids[offset:offset + CATALOG_BATCH_SIZE])
        covered = {cid for batch in batches for cid in batch.parsed.card_ids}
        if covered != set(ids):
            raise CompletenessError("secondary catalog acquisition has incomplete discovery coverage")
        return DigimonCardIOCatalogAcquisition(
            discovery_url, fetched.payload_digest, tuple(ids), tuple(batches),
        )
