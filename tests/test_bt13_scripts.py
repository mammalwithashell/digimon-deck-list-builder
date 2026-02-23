"""Validate transpiled BT13 generated scripts can be imported and instantiated."""

import importlib.util
import json
import os
import pytest
import sys
import types

# Ensure project root is on path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

BT13_SCRIPTS_DIR = os.path.join(
    os.path.dirname(__file__), "..", "digimon_gym", "engine", "data", "scripts", "generated", "bt13"
)


def get_bt13_script_modules():
    """Yield (module_name, class_name) for each BT13 generated script."""
    for fname in sorted(os.listdir(BT13_SCRIPTS_DIR)):
        if fname.startswith("bt13_") and fname.endswith(".py"):
            module_name = fname[:-3]
            class_name = module_name.replace("bt13", "BT13")
            yield module_name, class_name


BT13_SCRIPTS = list(get_bt13_script_modules())


def load_bt13_generated_module(module_name: str):
    package_name = "digimon_gym.engine.data.scripts.bt13"
    if package_name not in sys.modules:
        pkg = types.ModuleType(package_name)
        pkg.__path__ = [BT13_SCRIPTS_DIR]  # type: ignore[attr-defined]
        sys.modules[package_name] = pkg

    full_name = f"{package_name}.{module_name}"
    module_path = os.path.join(BT13_SCRIPTS_DIR, f"{module_name}.py")
    spec = importlib.util.spec_from_file_location(full_name, module_path)
    assert spec and spec.loader, f"Failed to build module spec for {module_path}"
    module = importlib.util.module_from_spec(spec)
    sys.modules[full_name] = module
    spec.loader.exec_module(module)
    return module


@pytest.mark.parametrize("module_name,class_name", BT13_SCRIPTS, ids=[m for m, _ in BT13_SCRIPTS])
def test_bt13_generated_script_imports(module_name, class_name):
    module = load_bt13_generated_module(module_name)
    script_class = getattr(module, class_name)
    instance = script_class()
    assert hasattr(instance, "get_card_effects")


@pytest.mark.parametrize("module_name,class_name", BT13_SCRIPTS, ids=[m for m, _ in BT13_SCRIPTS])
def test_bt13_generated_script_returns_effects(module_name, class_name):
    module = load_bt13_generated_module(module_name)
    script_class = getattr(module, class_name)
    instance = script_class()
    effects = instance.get_card_effects(None)
    assert isinstance(effects, list)


def test_bt13_generated_script_count():
    count = len([f for f in os.listdir(BT13_SCRIPTS_DIR) if f.startswith("bt13_") and f.endswith(".py")])
    assert count == 112, f"Expected 112 BT13 generated scripts, found {count}"


def test_bt13_cards_in_database():
    cards_path = os.path.join(os.path.dirname(__file__), "..", "digimon_gym", "engine", "data", "cards.json")
    with open(cards_path, encoding="utf-8") as f:
        cards = json.load(f)
    if isinstance(cards, dict):
        bt13_cards = [c for cid, c in cards.items() if cid.startswith("BT13-")]
    else:
        bt13_cards = [c for c in cards if c["card_id"].startswith("BT13-")]
    assert len(bt13_cards) == 112, f"Expected 112 BT13 cards in database, found {len(bt13_cards)}"


def test_bt13_manual_gap_scripts_present():
    for module_name, class_name in [
        ("bt13_022", "BT13_022"),
        ("bt13_024", "BT13_024"),
        ("bt13_066", "BT13_066"),
    ]:
        module = load_bt13_generated_module(module_name)
        script_class = getattr(module, class_name)
        effects = script_class().get_card_effects(None)
        assert isinstance(effects, list)
