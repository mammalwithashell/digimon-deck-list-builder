"""Validate that all transpiled BT14 card scripts can be imported and instantiated."""

import importlib
import os
import pytest
import sys

# Ensure project root is on path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

BT14_SCRIPTS_DIR = os.path.join(
    os.path.dirname(__file__), "..", "digimon_gym", "engine", "data", "scripts", "bt14"
)


def get_bt14_script_modules():
    """Yield (module_name, class_name) for each BT14 script."""
    for fname in sorted(os.listdir(BT14_SCRIPTS_DIR)):
        if fname.startswith("bt14_") and fname.endswith(".py"):
            module_name = fname[:-3]
            class_name = module_name.upper().replace("BT14", "BT14")  # bt14_001 -> BT14_001
            # Fix: class_name should be BT14_XXX
            class_name = "BT14_" + module_name.split("_", 1)[1].upper()
            # Handle: bt14_001 -> BT14_001, bt14_100 -> BT14_100
            class_name = module_name.replace("bt14", "BT14")
            yield module_name, class_name


BT14_SCRIPTS = list(get_bt14_script_modules())


@pytest.mark.parametrize("module_name,class_name", BT14_SCRIPTS, ids=[m for m, _ in BT14_SCRIPTS])
def test_bt14_script_imports(module_name, class_name):
    """Each BT14 script should be importable and have a CardScript subclass."""
    module_path = f"digimon_gym.engine.data.scripts.bt14.{module_name}"
    module = importlib.import_module(module_path)
    script_class = getattr(module, class_name)
    instance = script_class()
    assert hasattr(instance, "get_card_effects")


@pytest.mark.parametrize("module_name,class_name", BT14_SCRIPTS, ids=[m for m, _ in BT14_SCRIPTS])
def test_bt14_script_returns_effects(module_name, class_name):
    """Each script's get_card_effects should return a list without errors."""
    module_path = f"digimon_gym.engine.data.scripts.bt14.{module_name}"
    module = importlib.import_module(module_path)
    script_class = getattr(module, class_name)
    instance = script_class()
    # Pass None as card (scripts shouldn't crash on None for basic structure)
    effects = instance.get_card_effects(None)
    assert isinstance(effects, list)


def test_bt14_script_count():
    """We should have 94 BT14 script files."""
    count = len([f for f in os.listdir(BT14_SCRIPTS_DIR) if f.startswith("bt14_") and f.endswith(".py")])
    assert count == 94, f"Expected 94 BT14 scripts, found {count}"


def test_bt14_cards_in_database():
    """cards.json should contain all 102 BT14 cards."""
    import json
    cards_path = os.path.join(os.path.dirname(__file__), "..", "digimon_gym", "engine", "data", "cards.json")
    with open(cards_path) as f:
        cards = json.load(f)
    bt14_cards = [c for c in cards if c["card_id"].startswith("BT14")]
    assert len(bt14_cards) == 102, f"Expected 102 BT14 cards in database, found {len(bt14_cards)}"
