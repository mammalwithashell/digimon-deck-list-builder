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
