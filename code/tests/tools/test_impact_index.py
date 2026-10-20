from __future__ import annotations

import json
from pathlib import Path

from tools.impact_index import DEFAULT_CARDS_DIR, DEFAULT_OUT, build_impact_index, write_index


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


def test_trigger_if_and_source_trash_protection_predicates_are_indexed(tmp_path: Path):
    # `trigger_if:` (the trigger qualifier) and `source_card_trash_protection:`
    # are clause-level predicate containers; their keys must reach the index.
    cards = tmp_path / "cards"
    write_card(
        cards,
        "bt99",
        "BT99-004",
        """
card: BT99-004
effects:
  - when: on_any_digimon_played
    trigger_if:
      all_of:
        - event_target_owner: you
        - event_target_dp_lte: 4000
    process:
      - gain_memory: 1
  - kind: static
    source_card_trash_protection: { name_is: X Antibody }
""",
    )

    index = build_impact_index(cards)

    assert index["predicates"]["event_target_owner"] == ["BT99-004"]
    assert index["predicates"]["event_target_dp_lte"] == ["BT99-004"]
    assert index["predicates"]["name_is"] == ["BT99-004"]


def test_committed_index_lists_ad1_001_trigger_if_predicate():
    # AD1-001's `event_target_owner` appears only under `trigger_if:`.
    index = build_impact_index(DEFAULT_CARDS_DIR)
    assert "event_target_owner" in index["cards"]["AD1-001"]["predicates"]
    assert "AD1-001" in index["predicates"]["event_target_owner"]
    committed = json.loads(DEFAULT_OUT.read_text(encoding="utf-8"))
    assert "event_target_owner" in committed["cards"]["AD1-001"]["predicates"]
