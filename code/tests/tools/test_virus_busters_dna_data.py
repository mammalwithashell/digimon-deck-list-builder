"""Keep the printed WereGarurumon DNA recipe intact across data ingestion."""
import json
from pathlib import Path

from tools.ingest_cards import apply_overrides

ROOT = Path(__file__).resolve().parents[3]


def test_weregarurumon_override_repairs_bad_upstream_recipe_and_matches_split_data():
    overrides = json.loads((ROOT / "data/card_overrides.json").read_text(encoding="utf-8"))
    cards = {"EX12-032": {
        "xros_req": "[DNA Digivolve] Blue/Yellow Lv.5 + Purple/Red Lv.5: Cost 0",
        "dna_costs": [{"requirement1": {"level": 5}, "requirement2": {"level": 5}}],
    }}
    apply_overrides(cards, overrides={"EX12-032": overrides["EX12-032"]})
    fixed = cards["EX12-032"]
    recipe = fixed["dna_costs"][0]
    assert recipe["requirement1"]["level"] == recipe["requirement2"]["level"] == 4
    assert recipe["requirement1"]["card_colors"] == ["Blue", "Yellow"]
    assert recipe["requirement2"]["card_colors"] == ["Purple", "Red"]
    assert recipe["memory_cost"] == 0
    central = json.loads((ROOT / "data/cards.json").read_text(encoding="utf-8"))["EX12-032"]
    split = json.loads((ROOT / "code/digimon-engine/cards/ex12/EX12-032.json").read_text(encoding="utf-8"))
    assert central == split
    for field in ("xros_req", "dna_costs"):
        assert fixed[field] == central[field]
