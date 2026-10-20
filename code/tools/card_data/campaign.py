"""Versioned campaign schema, state derivation, and lane discrepancies."""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import StrEnum
from typing import Any, Mapping

from .errors import ValidationError
from .models import SHA256_RE
from .registry import is_gameplay_card_id
from .serialization import canonicalize, content_digest, natural_card_id_key


CAMPAIGN_SCHEMA_VERSION = 1
DONE_VERDICTS = frozenset({"IMPLEMENTED", "AUDITED-OK"})
PARTIAL_VERDICTS = frozenset({"PARTIAL", "INCOMPLETE", "BLOCKED-PARTIAL"})


class CampaignState(StrEnum):
    SKIP = "SKIP"
    DONE = "DONE"
    BLOCKED = "BLOCKED"
    PARTIAL = "PARTIAL"
    AUDIT = "AUDIT"
    IMPLEMENT = "IMPLEMENT"


STATE_PRECEDENCE = (
    CampaignState.SKIP,
    CampaignState.DONE,
    CampaignState.BLOCKED,
    CampaignState.PARTIAL,
    CampaignState.AUDIT,
    CampaignState.IMPLEMENT,
)


@dataclass(frozen=True, slots=True)
class BehavioralEvidence:
    evidence_id: str
    card_ids: tuple[str, ...]
    test_targets: tuple[str, ...]
    command: str
    profile: str
    engine_revision: str
    source_revision: str
    result: str
    artifact_digest: str
    canonical_data_fingerprint: str
    yaml_fingerprint: str
    test_fingerprint: str
    current: bool

    @property
    def passing(self) -> bool:
        return (
            self.result == "passed"
            and self.current
            and bool(self.command)
            and bool(self.profile)
            and len(self.artifact_digest) == 64
        )


@dataclass(frozen=True, slots=True)
class CampaignInputs:
    card_id: str
    canonical_admission: str = "admitted"
    canonical_record_digest: str = ""
    production_yaml_paths: tuple[str, ...] = ()
    example_yaml_paths: tuple[str, ...] = ()
    yaml_digests: Mapping[str, str] = field(default_factory=dict)
    runtime_registered: bool = False
    verdict_status: str | None = None
    verdict_current: bool = False
    historical_verdict: bool = False
    behavioral_evidence: tuple[BehavioralEvidence, ...] = ()
    ignored_tests: tuple[str, ...] = ()
    blocker_ids: tuple[str, ...] = ()
    reviewed_skip_reason: str | None = None
    deck_usage: tuple[Mapping[str, object], ...] = ()

    @property
    def has_production_evidence(self) -> bool:
        return bool(
            self.production_yaml_paths
            or self.runtime_registered
            or self.verdict_status
            or self.historical_verdict
        )

    @property
    def current_passing_evidence(self) -> bool:
        """Return whether the latest applicable ledger row is a current pass.

        ``behavioral_evidence`` is kept in append-only ledger order by the
        campaign adapter.  Every row in this per-card tuple is applicable to
        ``card_id``, so an older pass cannot outvote a later failed/not-run or
        fingerprint-stale result.
        """

        if not self.behavioral_evidence:
            return False
        latest = self.behavioral_evidence[-1]
        return latest.passing and self.card_id in latest.card_ids


def derive_campaign_state(inputs: CampaignInputs) -> CampaignState:
    """Apply the ordered exhaustive state table from the campaign spec."""

    contradictory_skip = bool(
        inputs.reviewed_skip_reason
        and (
            inputs.production_yaml_paths
            or inputs.runtime_registered
            or inputs.verdict_status
            or inputs.historical_verdict
            or inputs.behavioral_evidence
        )
    )
    if contradictory_skip:
        raise ValidationError(
            "reviewed SKIP contradicts implementation evidence",
            card_id=inputs.card_id,
        )
    if inputs.reviewed_skip_reason:
        return CampaignState.SKIP

    verdict_accepted = (
        inputs.verdict_current and inputs.verdict_status in DONE_VERDICTS
    )
    if (
        inputs.production_yaml_paths
        and inputs.runtime_registered
        and verdict_accepted
        and inputs.current_passing_evidence
        and not inputs.ignored_tests
        and not inputs.blocker_ids
    ):
        return CampaignState.DONE
    if inputs.blocker_ids:
        return CampaignState.BLOCKED
    if (
        inputs.has_production_evidence
        and inputs.verdict_current
        and inputs.verdict_status in PARTIAL_VERDICTS
    ):
        return CampaignState.PARTIAL
    if inputs.has_production_evidence:
        return CampaignState.AUDIT
    return CampaignState.IMPLEMENT


@dataclass(frozen=True, slots=True)
class CampaignRecord:
    card_id: str
    state: CampaignState
    inputs: CampaignInputs
    discrepancies: tuple[str, ...] = ()

    def __post_init__(self) -> None:
        if self.inputs.card_id != self.card_id:
            raise ValidationError("campaign record/input card IDs do not match", card_id=self.card_id)
        expected = derive_campaign_state(self.inputs)
        if self.state is not expected:
            raise ValidationError(
                "campaign record state does not match ordered derivation",
                card_id=self.card_id,
                details={"expected": expected.value, "actual": self.state.value},
            )


@dataclass(frozen=True, slots=True)
class CampaignInputRevision:
    canonical_revision: str
    source_manifest_digest: str
    yaml_tree_digest: str
    runtime_registry_digest: str
    verdict_digest: str
    behavioral_evidence_digest: str
    ignored_test_digest: str
    blocker_owner_digest: str
    deck_usage_digest: str
    engine_revision: str
    test_tree_digest: str

    def __post_init__(self) -> None:
        for field_name in (
            "canonical_revision",
            "source_manifest_digest",
            "yaml_tree_digest",
            "runtime_registry_digest",
            "verdict_digest",
            "behavioral_evidence_digest",
            "ignored_test_digest",
            "blocker_owner_digest",
            "deck_usage_digest",
            "engine_revision",
            "test_tree_digest",
        ):
            value = getattr(self, field_name)
            if not isinstance(value, str) or not SHA256_RE.fullmatch(value):
                raise ValidationError(
                    "campaign input revision requires lowercase sha256 digests",
                    field_path=f"input_revision.{field_name}",
                )

    @property
    def revision(self) -> str:
        return content_digest(self)


@dataclass(frozen=True, slots=True)
class CampaignSummary:
    canonical_real_card_count: int
    state_counts: Mapping[str, int]
    non_done_card_ids: tuple[str, ...]

    def __post_init__(self) -> None:
        expected_states = {state.value for state in CampaignState}
        if set(self.state_counts) != expected_states:
            raise ValidationError("campaign summary must enumerate every state")
        if sum(self.state_counts.values()) != self.canonical_real_card_count:
            raise ValidationError("campaign summary states do not reconcile to its universe")


@dataclass(frozen=True, slots=True)
class CampaignManifest:
    schema_version: int
    input_revision: CampaignInputRevision
    records: Mapping[str, CampaignRecord]
    summary: CampaignSummary
    lane_discrepancies: "LaneDiscrepancies"

    def __post_init__(self) -> None:
        if self.schema_version != CAMPAIGN_SCHEMA_VERSION:
            raise ValidationError("unsupported campaign manifest schema version")
        if len(self.records) != self.summary.canonical_real_card_count:
            raise ValidationError("campaign record count does not match canonical universe")
        record_ids = tuple(self.records)
        for card_id, record in self.records.items():
            if (
                not isinstance(card_id, str)
                or card_id != card_id.upper()
                or not is_gameplay_card_id(card_id)
            ):
                raise ValidationError(
                    "campaign records must use canonical gameplay-card IDs",
                    field_path="records",
                    details={"card_id": card_id},
                )
            if card_id != record.card_id:
                raise ValidationError("campaign mapping key does not match record card_id")
        if record_ids != tuple(sorted(set(record_ids), key=natural_card_id_key)):
            raise ValidationError(
                "campaign records must be unique and canonically ordered",
                field_path="records",
            )
        expected_counts = {state.value: 0 for state in CampaignState}
        expected_non_done: list[str] = []
        for card_id in sorted(self.records, key=natural_card_id_key):
            state = self.records[card_id].state
            expected_counts[state.value] += 1
            if state is not CampaignState.DONE:
                expected_non_done.append(card_id)
        if dict(self.summary.state_counts) != expected_counts:
            raise ValidationError("campaign summary state counts do not match records")
        if tuple(self.summary.non_done_card_ids) != tuple(expected_non_done):
            raise ValidationError("campaign summary non-DONE IDs do not match records")
        canonical_ids = set(self.records)
        c_bound_memberships = {
            "data_only": self.lane_discrepancies.data_only,
            "registry_only": self.lane_discrepancies.registry_only,
            "example_runtime": self.lane_discrepancies.example_runtime,
            "missing_verdict": self.lane_discrepancies.missing_verdict,
            "stale_evidence": self.lane_discrepancies.stale_evidence,
            "ignored_test": self.lane_discrepancies.ignored_test,
            "generated_data_drift.missing_from_generated": (
                self.lane_discrepancies.generated_data_drift[
                    "missing_from_generated"
                ]
            ),
        }
        for field_name, card_ids in c_bound_memberships.items():
            outside_canonical = set(card_ids) - canonical_ids
            if outside_canonical:
                raise ValidationError(
                    "campaign discrepancy must be a subset of the canonical universe",
                    field_path=f"lane_discrepancies.{field_name}",
                    details={
                        "card_ids": sorted(
                            outside_canonical, key=natural_card_id_key
                        )
                    },
                )
        c_disjoint_memberships = {
            "secondary_only": self.lane_discrepancies.secondary_only,
            "yaml_only": self.lane_discrepancies.yaml_only,
            "generated_data_drift.extra_in_generated": (
                self.lane_discrepancies.generated_data_drift[
                    "extra_in_generated"
                ]
            ),
        }
        for field_name, card_ids in c_disjoint_memberships.items():
            canonical_overlap = set(card_ids) & canonical_ids
            if canonical_overlap:
                raise ValidationError(
                    "campaign discrepancy must be disjoint from the canonical universe",
                    field_path=f"lane_discrepancies.{field_name}",
                    details={
                        "card_ids": sorted(
                            canonical_overlap, key=natural_card_id_key
                        )
                    },
                )
        memberships = self.lane_discrepancies.membership_by_card()
        for card_id, record in self.records.items():
            if record.discrepancies != memberships.get(card_id, ()):
                raise ValidationError(
                    "campaign record discrepancies do not match lane summary"
                )

    @classmethod
    def create(
        cls,
        *,
        input_revision: CampaignInputRevision,
        inputs_by_card: Mapping[str, CampaignInputs],
        discrepancies_by_card: Mapping[str, tuple[str, ...]] | None = None,
        lane_discrepancies: "LaneDiscrepancies | None" = None,
    ) -> "CampaignManifest":
        lane_discrepancies = lane_discrepancies or LaneDiscrepancies.empty()
        discrepancies_by_card = (
            discrepancies_by_card
            if discrepancies_by_card is not None
            else lane_discrepancies.membership_by_card()
        )
        records = {
            card_id: CampaignRecord(
                card_id=card_id,
                state=derive_campaign_state(inputs_by_card[card_id]),
                inputs=inputs_by_card[card_id],
                discrepancies=tuple(sorted(discrepancies_by_card.get(card_id, ()))),
            )
            for card_id in sorted(inputs_by_card, key=natural_card_id_key)
        }
        counts = {state.value: 0 for state in CampaignState}
        for record in records.values():
            counts[record.state.value] += 1
        non_done = tuple(
            card_id for card_id, record in records.items() if record.state is not CampaignState.DONE
        )
        return cls(
            schema_version=CAMPAIGN_SCHEMA_VERSION,
            input_revision=input_revision,
            records=records,
            summary=CampaignSummary(len(records), counts, non_done),
            lane_discrepancies=lane_discrepancies,
        )

    def filter_set(self, prefix: str) -> "CampaignManifest":
        prefix = prefix.upper().rstrip("-") + "-"
        subset = {
            card_id: record.inputs
            for card_id, record in self.records.items()
            if card_id.startswith(prefix)
        }
        discrepancies = {
            card_id: self.records[card_id].discrepancies for card_id in subset
        }
        return CampaignManifest.create(
            input_revision=self.input_revision,
            inputs_by_card=subset,
            discrepancies_by_card=discrepancies,
            lane_discrepancies=self.lane_discrepancies.filter_set(prefix),
        )


def campaign_manifest_from_payload(payload: Mapping[str, Any]) -> CampaignManifest:
    """Reconstruct and validate a serialized campaign manifest.

    A matching self-declared content hash is not enough for a generated trust
    boundary.  Rebuilding the typed model reruns every record's ordered state
    derivation and the summary reconciliation, then an exact canonical-shape
    comparison rejects omitted, injected, or weakly typed fields.
    """

    body = {key: value for key, value in payload.items() if key != "manifest_revision"}
    try:
        revision_payload = body["input_revision"]
        records_payload = body["records"]
        summary_payload = body["summary"]
        discrepancy_payload = body["lane_discrepancies"]
        if not isinstance(revision_payload, Mapping):
            raise TypeError("input_revision must be an object")
        if not isinstance(records_payload, Mapping):
            raise TypeError("records must be an object")
        if not isinstance(summary_payload, Mapping):
            raise TypeError("summary must be an object")
        if not isinstance(discrepancy_payload, Mapping):
            raise TypeError("lane_discrepancies must be an object")

        input_revision = CampaignInputRevision(**revision_payload)
        records: dict[str, CampaignRecord] = {}
        for card_id, raw_record in records_payload.items():
            if not isinstance(card_id, str) or not isinstance(raw_record, Mapping):
                raise TypeError("campaign records must be keyed objects")
            raw_inputs = raw_record["inputs"]
            if not isinstance(raw_inputs, Mapping):
                raise TypeError("campaign record inputs must be an object")
            raw_evidence = raw_inputs.get("behavioral_evidence", [])
            if not isinstance(raw_evidence, list):
                raise TypeError("behavioral_evidence must be an array")
            evidence: list[BehavioralEvidence] = []
            for raw_row in raw_evidence:
                if not isinstance(raw_row, Mapping):
                    raise TypeError("behavioral evidence row must be an object")
                if not isinstance(raw_row.get("current"), bool):
                    raise TypeError("behavioral evidence current must be boolean")
                evidence.append(
                    BehavioralEvidence(
                        evidence_id=raw_row["evidence_id"],
                        card_ids=tuple(raw_row["card_ids"]),
                        test_targets=tuple(raw_row["test_targets"]),
                        command=raw_row["command"],
                        profile=raw_row["profile"],
                        engine_revision=raw_row["engine_revision"],
                        source_revision=raw_row["source_revision"],
                        result=raw_row["result"],
                        artifact_digest=raw_row["artifact_digest"],
                        canonical_data_fingerprint=raw_row[
                            "canonical_data_fingerprint"
                        ],
                        yaml_fingerprint=raw_row["yaml_fingerprint"],
                        test_fingerprint=raw_row["test_fingerprint"],
                        current=raw_row["current"],
                    )
                )
            inputs = CampaignInputs(
                card_id=raw_inputs["card_id"],
                canonical_admission=raw_inputs["canonical_admission"],
                canonical_record_digest=raw_inputs["canonical_record_digest"],
                production_yaml_paths=tuple(raw_inputs["production_yaml_paths"]),
                example_yaml_paths=tuple(raw_inputs["example_yaml_paths"]),
                yaml_digests=dict(raw_inputs["yaml_digests"]),
                runtime_registered=raw_inputs["runtime_registered"],
                verdict_status=raw_inputs["verdict_status"],
                verdict_current=raw_inputs["verdict_current"],
                historical_verdict=raw_inputs["historical_verdict"],
                behavioral_evidence=tuple(evidence),
                ignored_tests=tuple(raw_inputs["ignored_tests"]),
                blocker_ids=tuple(raw_inputs["blocker_ids"]),
                reviewed_skip_reason=raw_inputs["reviewed_skip_reason"],
                deck_usage=tuple(raw_inputs["deck_usage"]),
            )
            records[card_id] = CampaignRecord(
                card_id=raw_record["card_id"],
                state=CampaignState(raw_record["state"]),
                inputs=inputs,
                discrepancies=tuple(raw_record["discrepancies"]),
            )
        summary = CampaignSummary(
            canonical_real_card_count=summary_payload["canonical_real_card_count"],
            state_counts=dict(summary_payload["state_counts"]),
            non_done_card_ids=tuple(summary_payload["non_done_card_ids"]),
        )
        lane_discrepancies = LaneDiscrepancies(
            secondary_only=tuple(discrepancy_payload["secondary_only"]),
            data_only=tuple(discrepancy_payload["data_only"]),
            yaml_only=tuple(discrepancy_payload["yaml_only"]),
            registry_only=tuple(discrepancy_payload["registry_only"]),
            example_runtime=tuple(discrepancy_payload["example_runtime"]),
            duplicate_yaml={
                card_id: tuple(paths)
                for card_id, paths in discrepancy_payload["duplicate_yaml"].items()
            },
            missing_verdict=tuple(discrepancy_payload["missing_verdict"]),
            stale_evidence=tuple(discrepancy_payload["stale_evidence"]),
            ignored_test=tuple(discrepancy_payload["ignored_test"]),
            generated_data_drift={
                name: tuple(card_ids)
                for name, card_ids in discrepancy_payload[
                    "generated_data_drift"
                ].items()
            },
        )
        manifest = CampaignManifest(
            schema_version=body["schema_version"],
            input_revision=input_revision,
            records=records,
            summary=summary,
            lane_discrepancies=lane_discrepancies,
        )
    except (KeyError, TypeError, ValueError) as exc:
        raise ValidationError(
            "campaign manifest does not match the typed schema",
            details={"error": str(exc)},
        ) from exc
    if canonicalize(manifest) != canonicalize(body):
        raise ValidationError("campaign manifest contains non-canonical schema content")
    return manifest


@dataclass(frozen=True, slots=True)
class LaneSets:
    secondary_observations: frozenset[str]
    canonical_cards: frozenset[str]
    generated_cards: frozenset[str]
    production_yaml: frozenset[str]
    example_yaml: frozenset[str]
    raw_runtime_registry: frozenset[str]
    verdicts: frozenset[str]
    production_yaml_paths: Mapping[str, tuple[str, ...]] = field(default_factory=dict)
    stale_evidence: frozenset[str] = frozenset()
    ignored_tests: frozenset[str] = frozenset()


@dataclass(frozen=True, slots=True)
class LaneDiscrepancies:
    secondary_only: tuple[str, ...]
    data_only: tuple[str, ...]
    yaml_only: tuple[str, ...]
    registry_only: tuple[str, ...]
    example_runtime: tuple[str, ...]
    duplicate_yaml: Mapping[str, tuple[str, ...]]
    missing_verdict: tuple[str, ...]
    stale_evidence: tuple[str, ...]
    ignored_test: tuple[str, ...]
    generated_data_drift: Mapping[str, tuple[str, ...]]

    def __post_init__(self) -> None:
        id_fields = (
            "secondary_only",
            "data_only",
            "yaml_only",
            "registry_only",
            "example_runtime",
            "missing_verdict",
            "stale_evidence",
            "ignored_test",
        )
        for field_name in id_fields:
            self._validate_card_id_tuple(field_name, getattr(self, field_name))

        if not isinstance(self.duplicate_yaml, Mapping):
            raise ValidationError(
                "campaign duplicate-YAML discrepancies must be an object",
                field_path="lane_discrepancies.duplicate_yaml",
            )
        for card_id in self.duplicate_yaml:
            self._validate_gameplay_card_id("duplicate_yaml", card_id)

        expected_drift_keys = {
            "missing_from_generated",
            "extra_in_generated",
        }
        if (
            not isinstance(self.generated_data_drift, Mapping)
            or set(self.generated_data_drift) != expected_drift_keys
        ):
            raise ValidationError(
                "campaign generated-data drift must use the exact discrepancy vocabulary",
                field_path="lane_discrepancies.generated_data_drift",
                details={"expected_keys": sorted(expected_drift_keys)},
            )
        for field_name in sorted(expected_drift_keys):
            self._validate_card_id_tuple(
                f"generated_data_drift.{field_name}",
                self.generated_data_drift[field_name],
            )

    @staticmethod
    def _validate_gameplay_card_id(field_name: str, card_id: object) -> None:
        if (
            not isinstance(card_id, str)
            or card_id != card_id.upper()
            or not is_gameplay_card_id(card_id)
        ):
            raise ValidationError(
                "campaign discrepancy IDs must be canonical gameplay-card strings",
                field_path=f"lane_discrepancies.{field_name}",
                details={"card_id": card_id},
            )

    @classmethod
    def _validate_card_id_tuple(
        cls,
        field_name: str,
        card_ids: object,
    ) -> None:
        if not isinstance(card_ids, tuple):
            raise ValidationError(
                "campaign discrepancy membership must be a tuple",
                field_path=f"lane_discrepancies.{field_name}",
            )
        for card_id in card_ids:
            cls._validate_gameplay_card_id(field_name, card_id)
        expected = tuple(sorted(set(card_ids), key=natural_card_id_key))
        if card_ids != expected:
            raise ValidationError(
                "campaign discrepancy IDs must be unique and canonically ordered",
                field_path=f"lane_discrepancies.{field_name}",
            )

    @classmethod
    def empty(cls) -> "LaneDiscrepancies":
        return cls(
            secondary_only=(),
            data_only=(),
            yaml_only=(),
            registry_only=(),
            example_runtime=(),
            duplicate_yaml={},
            missing_verdict=(),
            stale_evidence=(),
            ignored_test=(),
            generated_data_drift={
                "missing_from_generated": (),
                "extra_in_generated": (),
            },
        )

    def filter_set(self, prefix: str) -> "LaneDiscrepancies":
        normalized = prefix.upper().rstrip("-") + "-"

        def ids(values: tuple[str, ...]) -> tuple[str, ...]:
            return tuple(card_id for card_id in values if card_id.startswith(normalized))

        return LaneDiscrepancies(
            secondary_only=ids(self.secondary_only),
            data_only=ids(self.data_only),
            yaml_only=ids(self.yaml_only),
            registry_only=ids(self.registry_only),
            example_runtime=ids(self.example_runtime),
            duplicate_yaml={
                card_id: paths
                for card_id, paths in self.duplicate_yaml.items()
                if card_id.startswith(normalized)
            },
            missing_verdict=ids(self.missing_verdict),
            stale_evidence=ids(self.stale_evidence),
            ignored_test=ids(self.ignored_test),
            generated_data_drift={
                name: ids(tuple(card_ids))
                for name, card_ids in self.generated_data_drift.items()
            },
        )

    def membership_by_card(self) -> dict[str, tuple[str, ...]]:
        output: dict[str, list[str]] = {}
        for name in (
            "secondary_only",
            "data_only",
            "yaml_only",
            "registry_only",
            "example_runtime",
            "missing_verdict",
            "stale_evidence",
            "ignored_test",
        ):
            for card_id in getattr(self, name):
                output.setdefault(card_id, []).append(name)
        for card_id in self.duplicate_yaml:
            output.setdefault(card_id, []).append("duplicate_yaml")
        for card_id in self.generated_data_drift["missing_from_generated"]:
            output.setdefault(card_id, []).append("generated_data_drift")
        for card_id in self.generated_data_drift["extra_in_generated"]:
            output.setdefault(card_id, []).append("generated_data_drift")
        return {card_id: tuple(sorted(names)) for card_id, names in sorted(output.items())}


def compute_lane_discrepancies(
    lanes: LaneSets,
    *,
    strict_integrity: bool = True,
) -> LaneDiscrepancies:
    c = set(lanes.canonical_cards)
    y = set(lanes.production_yaml)
    e = set(lanes.example_yaml)
    r = set(lanes.raw_runtime_registry) & c
    v = set(lanes.verdicts)
    # ``cards.json`` deliberately retains marker-owned TEST/TST/TOKEN fixtures
    # for compatibility.  They are outside the canonical real-card universe
    # and therefore cannot be generated-data drift or affect campaign counts.
    d = {
        card_id
        for card_id in lanes.generated_cards
        if is_gameplay_card_id(card_id)
    }
    duplicate_yaml = {
        card_id: tuple(sorted(paths))
        for card_id, paths in lanes.production_yaml_paths.items()
        if len(paths) > 1
    }
    yaml_only = y - c
    if strict_integrity and (duplicate_yaml or yaml_only):
        raise ValidationError(
            "campaign lane integrity failed",
            details={
                "duplicate_yaml": duplicate_yaml,
                "yaml_only": sorted(yaml_only, key=natural_card_id_key),
            },
        )
    key = natural_card_id_key
    return LaneDiscrepancies(
        secondary_only=tuple(sorted(set(lanes.secondary_observations) - c, key=key)),
        data_only=tuple(sorted(c - (y | r | e), key=key)),
        yaml_only=tuple(sorted(yaml_only, key=key)),
        registry_only=tuple(sorted(r - y, key=key)),
        example_runtime=tuple(sorted((e & r) - y, key=key)),
        duplicate_yaml=duplicate_yaml,
        missing_verdict=tuple(sorted(((y | r) & c) - v, key=key)),
        stale_evidence=tuple(sorted(set(lanes.stale_evidence) & c, key=key)),
        ignored_test=tuple(sorted(set(lanes.ignored_tests) & c, key=key)),
        generated_data_drift={
            "missing_from_generated": tuple(sorted(c - d, key=key)),
            "extra_in_generated": tuple(sorted(d - c, key=key)),
        },
    )
