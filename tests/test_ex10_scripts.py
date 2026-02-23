"""Validate transpiled EX10 generated scripts can be imported and instantiated."""

import importlib.util
import json
import os
import pytest
import sys
import types

# Ensure project root is on path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

EX10_SCRIPTS_DIR = os.path.join(
    os.path.dirname(__file__), "..", "digimon_gym", "engine", "data", "scripts", "generated", "ex10"
)


def get_ex10_script_modules():
    """Yield (module_name, class_name) for each EX10 generated script."""
    for fname in sorted(os.listdir(EX10_SCRIPTS_DIR)):
        if fname.startswith("ex10_") and fname.endswith(".py"):
            module_name = fname[:-3]
            class_name = module_name.replace("ex10", "EX10")
            yield module_name, class_name


EX10_SCRIPTS = list(get_ex10_script_modules())


def load_ex10_generated_module(module_name: str):
    package_name = "digimon_gym.engine.data.scripts.ex10"
    if package_name not in sys.modules:
        pkg = types.ModuleType(package_name)
        pkg.__path__ = [EX10_SCRIPTS_DIR]  # type: ignore[attr-defined]
        sys.modules[package_name] = pkg

    full_name = f"{package_name}.{module_name}"
    module_path = os.path.join(EX10_SCRIPTS_DIR, f"{module_name}.py")
    spec = importlib.util.spec_from_file_location(full_name, module_path)
    assert spec and spec.loader, f"Failed to build module spec for {module_path}"
    module = importlib.util.module_from_spec(spec)
    sys.modules[full_name] = module
    spec.loader.exec_module(module)
    return module


@pytest.mark.parametrize("module_name,class_name", EX10_SCRIPTS, ids=[m for m, _ in EX10_SCRIPTS])
def test_ex10_generated_script_imports(module_name, class_name):
    module = load_ex10_generated_module(module_name)
    script_class = getattr(module, class_name)
    instance = script_class()
    assert hasattr(instance, "get_card_effects")


@pytest.mark.parametrize("module_name,class_name", EX10_SCRIPTS, ids=[m for m, _ in EX10_SCRIPTS])
def test_ex10_generated_script_returns_effects(module_name, class_name):
    module = load_ex10_generated_module(module_name)
    script_class = getattr(module, class_name)
    instance = script_class()
    effects = instance.get_card_effects(None)
    assert isinstance(effects, list)


def test_ex10_generated_script_count():
    count = len([f for f in os.listdir(EX10_SCRIPTS_DIR) if f.startswith("ex10_") and f.endswith(".py")])
    assert count == 74, f"Expected 74 EX10 generated scripts, found {count}"


def test_ex10_cards_in_database():
    cards_path = os.path.join(os.path.dirname(__file__), "..", "digimon_gym", "engine", "data", "cards.json")
    with open(cards_path, encoding="utf-8") as f:
        cards = json.load(f)
    if isinstance(cards, dict):
        ex10_cards = [c for cid, c in cards.items() if cid.startswith("EX10-")]
    else:
        ex10_cards = [c for c in cards if c["card_id"].startswith("EX10-")]
    assert len(ex10_cards) == 74, f"Expected 74 EX10 cards in database, found {len(ex10_cards)}"
