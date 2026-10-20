"""The cutover review must cover every current gameplay metadata surface."""
import pytest
from tools.card_data.compatibility_diff import diff_runtime_fields, require_approved_runtime_diffs
from tools.card_data.errors import AdjudicationError

@pytest.mark.parametrize("field,old", [
    ("card_name_eng", "Named Digimon"),
    ("dual", {"option": {"use_cost": 2}}),
    ("link_effect_description_eng", "[Your Turn] Gain memory."),
    ("max_count_in_deck", 50),
    ("dna_costs", [{"memory_cost": 0}]),
    ("digixros_costs", [{"different_levels": True}]),
])
def test_runtime_field_removal_requires_review(field, old):
    diffs = diff_runtime_fields({"BT26-033": {field: old}}, {"BT26-033": {}}, {})
    assert len(diffs) == 1
    assert diffs[0].field_path == field
    assert diffs[0].old_value == old
    assert diffs[0].new_value is None
    assert not diffs[0].approved
    with pytest.raises(AdjudicationError):
        require_approved_runtime_diffs(diffs)


def _projected_dual_diff():
    from copy import deepcopy
    from dataclasses import replace
    from pathlib import Path
    from tools.card_data.generate import project_canonical_card
    from tools.card_data.models import SourceName, SourceObservation, FieldProvenance, AuthorityClass
    from tools.card_data.normalization import normalize_card
    from tools.card_data.serialization import content_digest
    from tools.card_data.sources.official import parse_card_list

    payload = (Path(__file__).parents[1] / "fixtures/card_data/official/bt25_dual.html").read_bytes()
    (printing,) = parse_card_list(payload, source_url="https://example.test/official", category_id="522036")
    draft = SourceObservation(schema_version=1, observation_id="0" * 64, source=SourceName.OFFICIAL,
        source_record_key="522036:BT25-043:BT25-043", source_url="https://example.test/official",
        raw_payload_digest="a" * 64, parser_version="fixture-v1", source_schema_version="fixture-v1",
        first_acquired_at="2026-10-11T00:00:00Z", extracted=printing.extracted)
    observation = replace(draft, observation_id=content_digest(draft.identity_payload()))
    card = normalize_card("BT25-043", [observation])
    unrelated = replace(draft, raw_payload_digest="b" * 64)
    unrelated = replace(unrelated, observation_id=content_digest(unrelated.identity_payload()))
    provenance = dict(card.provenance)
    provenance["card_name_eng"] = FieldProvenance(selected_observation_id=unrelated.observation_id,
        observation_ids=(unrelated.observation_id,), authority=AuthorityClass.OFFICIAL)
    card = replace(card, provenance=provenance)
    new = project_canonical_card(card)
    old = deepcopy(new)
    old["dual"]["option"]["effect_text"] = "Old Option effect"
    (diff,) = diff_runtime_fields({card.card_id: old}, {card.card_id: new}, {card.card_id: card})
    return card, diff, observation, unrelated


def test_projected_dual_diff_carries_faces_evidence_and_accepts_exact_review():
    from tools.card_data.compatibility_diff import approve_runtime_diffs
    from tools.card_data.models import MigrationAdjudication, AdjudicationKind

    card, diff, observation, unrelated = _projected_dual_diff()
    assert diff.field_path == "dual"
    assert diff.provenance_observation_ids == card.provenance["faces"].observation_ids
    assert unrelated.observation_id not in diff.provenance_observation_ids
    approval = MigrationAdjudication(schema_version=1, adjudication_id="dual-reviewed",
        kind=AdjudicationKind.RUNTIME_FIELD_CUTOVER, card_id=card.card_id, field_path="dual",
        old_input_revision="c" * 64, new_input_revision="d" * 64,
        old_digest=diff.old_digest, new_digest=diff.new_digest,
        old_value=diff.old_value, new_value=diff.new_value,
        provenance_observation_ids=(observation.observation_id,),
        reason="Reviewed fixture Option face change", reviewer="fixture", reviewed_at="2026-10-11")
    (checked,) = approve_runtime_diffs([diff], corrections=[], adjudications=[approval],
        observations={observation.observation_id: observation, unrelated.observation_id: unrelated},
        old_input_revision="c" * 64, new_input_revision="d" * 64)
    assert checked.approved


@pytest.mark.parametrize("mismatch", ["unrelated_evidence", "stale_value", "stale_revision"])
def test_projected_dual_diff_rejects_mismatched_review(mismatch):
    from tools.card_data.compatibility_diff import approve_runtime_diffs
    from tools.card_data.models import MigrationAdjudication, AdjudicationKind

    card, diff, observation, unrelated = _projected_dual_diff()
    approval = MigrationAdjudication(schema_version=1, adjudication_id="dual-mismatch",
        kind=AdjudicationKind.RUNTIME_FIELD_CUTOVER, card_id=card.card_id, field_path="dual",
        old_input_revision="c" * 64, new_input_revision=("e" if mismatch == "stale_revision" else "d") * 64,
        old_digest=diff.old_digest, new_digest="f" * 64 if mismatch == "stale_value" else diff.new_digest,
        provenance_observation_ids=((unrelated if mismatch == "unrelated_evidence" else observation).observation_id,),
        reason="Negative fixture", reviewer="fixture", reviewed_at="2026-10-11")
    (checked,) = approve_runtime_diffs([diff], corrections=[], adjudications=[approval],
        observations={observation.observation_id: observation, unrelated.observation_id: unrelated},
        old_input_revision="c" * 64, new_input_revision="d" * 64)
    assert not checked.approved


def test_dual_provenance_uses_only_projection_dependencies_and_nested_corrections():
    from dataclasses import replace
    from tools.card_data.models import FieldProvenance, AuthorityClass

    card, diff, observation, unrelated = _projected_dual_diff()
    provenance = dict(card.provenance)
    expected = {observation.observation_id}
    for index, path in enumerate(("type_eng", "evo_costs", "faces.1.effect_text"), start=1):
        evidence_id = str(index) * 64
        provenance[path] = FieldProvenance(selected_observation_id=evidence_id,
            observation_ids=(evidence_id,), authority=AuthorityClass.ADJUDICATED,
            adjudicated=True)
        expected.add(evidence_id)
    card = replace(card, provenance=provenance)
    (checked,) = diff_runtime_fields({card.card_id: {"dual": diff.old_value}},
        {card.card_id: {"dual": diff.new_value}}, {card.card_id: card})
    assert checked.provenance_observation_ids == tuple(sorted(expected))
    assert unrelated.observation_id not in checked.provenance_observation_ids


@pytest.mark.parametrize("field_path", ["dual", "dual.option.effect_text"])
def test_dual_does_not_become_a_canonical_correction_target(field_path):
    from tools.card_data.models import ReviewedCorrection, ModelValidationError

    with pytest.raises(ModelValidationError, match="unsupported canonical field root"):
        ReviewedCorrection(schema_version=1, correction_id="not-canonical", card_id="BT25-043",
            field_path=field_path, value={}, evidence_observation_ids=("a" * 64,),
            reason="Negative fixture", reviewer="fixture", reviewed_at="2026-10-11")


@pytest.mark.parametrize("kind,path", [("canonical_field_selection", "dual"),
    ("runtime_field_cutover", "dual.option.effect_text")])
def test_dual_schema_exception_is_only_whole_runtime_field(kind, path):
    from tools.card_data.models import MigrationAdjudication, AdjudicationKind, ModelValidationError

    with pytest.raises(ModelValidationError, match="unsupported canonical field root"):
        MigrationAdjudication(schema_version=1, adjudication_id="not-runtime-dual",
            kind=AdjudicationKind(kind), card_id="BT25-043", field_path=path,
            old_input_revision="c" * 64, new_input_revision="d" * 64,
            old_digest="e" * 64, new_digest="f" * 64,
            reason="Negative fixture", reviewer="fixture", reviewed_at="2026-10-11")
