"""Image-reviewed EX10-016 exception across both retained Link text surfaces."""

from dataclasses import replace
import json
from pathlib import Path

import pytest

from tools.card_data.compatibility_diff import diff_runtime_fields
from tools.card_data.corrections import (
    apply_correction, correction_is_already_applied, load_reviewed_corrections,
)
from tools.card_data.errors import ValidationError
from tools.card_data.generate import project_canonical_card
from tools.card_data.live_reconcile import _observation_from_dict
from tools.card_data.models import AuthorityClass
from tools.card_data.normalization import normalize_card
from tools.card_data.serialization import observation_digest


FIXTURES = Path(__file__).parents[1] / "fixtures/card_data/official"
MAIN = "[Your Turn] [Once Per Turn] When this Digimon gets linked, you may suspend 1 of your opponent's Digimon."
ROUTE = "[Special Digivolution Condition] [Digivolve] Lv.2 w/[Appmon] trait: Cost 0"
LINK = "[When Attacking] By trashing 1 of this Digimon's link cards, suspend 2 of your opponent's Digimon."


def _inputs():
    raw = json.loads((FIXTURES / "ex10_016_link_observations.json").read_text(encoding="utf-8"))
    observations = [_observation_from_dict(row) for row in raw]
    assert len(observations) == 2
    assert all(observation_digest(o) == o.observation_id for o in observations)
    corrections = tuple(c for c in load_reviewed_corrections(
        FIXTURES / "ex10_016_link_reviewed_corrections.json"
    ) if c.card_id == "EX10-016")
    return normalize_card("EX10-016", observations), corrections, {o.observation_id: o for o in observations}


def test_image_review_changes_only_two_labeled_numbers_and_link_outcome():
    original, corrections, observations = _inputs()
    raw_before = json.dumps([o.extracted for o in observations.values()], sort_keys=True)
    fixed = original
    for correction in corrections:
        assert not correction_is_already_applied(fixed, correction, observations=observations)
        fixed = apply_correction(fixed, correction, observations=observations)
        assert correction_is_already_applied(fixed, correction, observations=observations)
    row = project_canonical_card(fixed)
    projected_main = row["effect_description_eng"].replace("\r\n", "\n")
    assert "[Link DP] +2000 DP" in row["effect_description_eng"]
    assert "[Appmon] trait: Cost 1 " in row["effect_description_eng"]
    assert "[Link DP] +2000 DP" in fixed.faces[0].effect_text
    assert "[Appmon] trait: Cost 1 " in fixed.faces[0].effect_text
    assert row["link_effect_description_eng"] == LINK
    assert row["play_cost"] == 3 and row["dp"] == 1000
    assert row["inherited_effect_description_eng"] == row["security_effect_description_eng"] == ""
    assert projected_main.startswith(MAIN + "\n")
    assert fixed.faces[0].effect_text.startswith(ROUTE + "\n" + MAIN + "\n")
    for before, after in [(original.fields["effect_description_eng"], projected_main),
                          (original.faces[0].effect_text, fixed.faces[0].effect_text)]:
        assert before.count("[Link DP] +3000 DP") == 1
        assert before.count("[Appmon] trait: Cost 2 ") == 1
        assert after == before.replace("[Link DP] +3000 DP", "[Link DP] +2000 DP").replace(
            "[Appmon] trait: Cost 2 ", "[Appmon] trait: Cost 1 ")
    assert fixed.printings == original.printings
    assert replace(fixed.faces[0], effect_text=original.faces[0].effect_text) == original.faces[0]
    changed = {k for k in fixed.fields if fixed.fields[k] != original.fields[k]}
    assert changed == {"effect_description_eng", "link_effect_description_eng"}
    assert json.dumps([o.extracted for o in observations.values()], sort_keys=True) == raw_before
    assert {c.field_path for c in corrections} == {
        "effect_description_eng", "faces.0.effect_text", "link_effect_description_eng"}
    for correction in corrections:
        provenance = fixed.provenance[correction.field_path]
        assert provenance.authority is AuthorityClass.ADJUDICATED
        assert provenance.observation_ids == tuple(sorted(observations))
        with pytest.raises(ValidationError, match="no-op"):
            apply_correction(fixed, correction, observations=observations)
    diffs = diff_runtime_fields({"EX10-016": project_canonical_card(original)},
                               {"EX10-016": row}, {"EX10-016": fixed})
    assert {d.field_path for d in diffs} == changed
    assert all(d.provenance_observation_ids == tuple(sorted(observations)) for d in diffs)


@pytest.mark.parametrize("failure", ["missing", "foreign"])
def test_link_review_rejects_missing_or_foreign_evidence(failure):
    card, corrections, observations = _inputs()
    evidence = dict(observations)
    if failure == "missing":
        evidence.clear()
    else:
        evidence = {k: replace(o, extracted=dict(o.extracted, card_id="EX10-017"))
                    for k, o in evidence.items()}
    for correction in corrections:
        with pytest.raises(ValidationError, match="missing or stale|another gameplay card"):
            apply_correction(card, correction, observations=evidence)
