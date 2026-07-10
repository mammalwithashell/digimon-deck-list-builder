from __future__ import annotations

import json
from pathlib import Path

from tools.impact_index import build_impact_index, write_index


def write_card(root: Path, set_id: str, card_id: str, body: str) -> Path:
    path = root / set_id / f"{card_id}.yaml"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(body, encoding="utf-8", newline="\n")
    return path


def test_build_impact_index_extracts_verbs_predicates_and_timings(tmp_path: Path):
    cards = tmp_path / "cards"
    write_card(
        cards,
        "bt99",
        "BT99-001",
        """
card: BT99-001
name: Example
kind: digimon
effects:
  - when: [on_play, when_digivolving]
    condition:
      all_of:
        - trait: Dinosaur
        - level_eq: 4
    process:
      - select_hand:
          of: you
          bind_as: pick
          filter:
            name_contains: Greymon
      - if:
          condition: { binding_is_present: pick }
          then:
            - gain_memory_fn: { base: 1 }
          else:
            - trash_from_hand_by_index: { of: you, index: pick }
""",
    )
    write_card(
        cards,
        "bt99",
        "BT99-002",
        """
card: BT99-002
name: Other
kind: option
effects:
  - kind: cost_reduction
    active_when: { your_turn: true }
    pay_cost:
      - delete_permanent: { target: source }
""",
    )

    index = build_impact_index(cards)

    assert index["verbs"]["select_hand"] == ["BT99-001"]
    assert index["verbs"]["gain_memory"] == ["BT99-001"]
    assert index["verbs"]["trash_from_hand_by_index"] == ["BT99-001"]
    assert index["verbs"]["delete_permanent"] == ["BT99-002"]
    assert index["predicates"]["trait_has"] == ["BT99-001"]
    assert index["predicates"]["binding_present"] == ["BT99-001"]
    assert index["predicates"]["name_contains"] == ["BT99-001"]
    assert index["predicates"]["your_turn"] == ["BT99-002"]
    assert index["timings"]["on_play"] == ["BT99-001"]
    assert index["timings"]["when_digivolving"] == ["BT99-001"]
    assert index["cards"]["BT99-001"]["path"] == "bt99/BT99-001.yaml"


def test_write_index_materializes_sorted_lf_json(tmp_path: Path):
    cards = tmp_path / "cards"
    write_card(
        cards,
        "bt99",
        "BT99-003",
        """
card: BT99-003
effects:
  - when: on_deletion
    process:
      - draw: { of: you, count: 1 }
""",
    )

    out = tmp_path / "impact" / "dsl_impact_index.json"
    write_index(build_impact_index(cards), out)

    raw = out.read_bytes()
    assert b"\r\n" not in raw
    parsed = json.loads(raw)
    assert parsed["verbs"]["draw"] == ["BT99-003"]
    assert parsed["timings"]["on_deletion"] == ["BT99-003"]
