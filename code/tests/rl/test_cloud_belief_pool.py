import copy
import pytest
from tools.prepare_cloud_belief_pool import validate_roles


def fixture():
    return {
        "train": [{"cards": ["A", "B"]}],
        "validation": [{"cards": ["C"]}],
        "test": [{"cards": ["D"]}],
        "pet_regression": [{"cards": ["B", "A"]}],
    }


def test_pet_regression_stays_separate_when_it_reuses_training_contents():
    assert validate_roles(fixture()) == {
        "train": 1,
        "validation": 1,
        "test": 1,
        "pet_regression": 1,
    }


@pytest.mark.parametrize(
    "left,right", [("train", "test"), ("train", "validation"), ("validation", "test")]
)
def test_explicit_cloud_splits_cannot_leak_by_card_order(left, right):
    p = fixture()
    p[right] = copy.deepcopy(p[left])
    p[right][0]["cards"].reverse()
    with pytest.raises(ValueError, match="leakage"):
        validate_roles(p)


def test_repeated_native_slots_are_not_distinct_validation_lists():
    p = fixture()
    p["test"] *= 2
    with pytest.raises(ValueError, match="duplicate"):
        validate_roles(p)
