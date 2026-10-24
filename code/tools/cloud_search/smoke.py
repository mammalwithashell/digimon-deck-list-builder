"""Exercise the published CPU image's native/Python boundary; no quality claim."""
from pathlib import Path
import json
import subprocess
import tempfile

import digimon_engine
import tools.anchored_eval_cli  # noqa: F401
import tools.export_onnx  # noqa: F401
import tools.run_selfplay_generation  # noqa: F401
from digimon_gym.agents.gauntlet import require_mechanics_ready, validate_admissible_deck
from model_contract import current_contract


def main():
    require_mechanics_ready()
    root = Path(__file__).resolve().parents[3]
    cards = root / "data/cards.json"
    native = set(json.loads(subprocess.check_output([
        "digimon-engine-cli", "--cards-json", str(cards), "pool"], text=True)))
    raw = json.loads(cards.read_text(encoding="utf-8"))
    available = set(raw) if isinstance(raw, dict) else {r["card_id"] for r in raw}
    assert native == set(digimon_engine.load_implemented_card_ids()) & available
    contract = current_contract("standard_lite_v2")
    library = json.loads((root / "data/deck_library.json").read_text(encoding="utf-8"))
    deck = None
    for row in library["archetypes"]["Medusamon"]["decklists"]:
        candidate = row["decklist"]
        if isinstance(candidate, str):
            candidate = json.loads(candidate)
        if not set(candidate) <= native:
            continue
        try:
            validate_admissible_deck(candidate)
        except ValueError:
            continue
        if digimon_engine.validate_deck(candidate).is_valid:
            deck = candidate
            break
    assert deck, "No admitted legal smoke deck"
    with tempfile.TemporaryDirectory() as temp:
        directory = Path(temp)
        pool = directory / "decks.json"
        pool.write_text(json.dumps([{"name": "ci-smoke", "cards": deck}]))
        output = directory / "native"
        subprocess.run([
            "digimon-engine-cli", "--pool", "implemented", "--cards-json", str(cards),
            "selfplay", "--uniform", "--tensor-profile", "standard_lite_v2",
            "--decks", str(pool), "--games", "2", "--sims", "8", "--worlds", "4",
            "--eval-batch-size", "4", "--decision-cap", "12", "--seed", "20261024",
            "--out", str(output)], check=True, timeout=120)
        manifest = json.loads((output / "manifest.json").read_text())
        assert manifest["games"] == 2 and manifest["rows"] > 0
        assert manifest["model_contract"] == contract
    print("SEARCH_CPU_IMAGE_OK: imports, compiled registry parity, admission, native shards")


if __name__ == "__main__":
    main()
