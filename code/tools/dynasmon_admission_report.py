"""Report exact Dynasmon list admission using the existing committed gate artifacts.

Regenerate card_admissibility and mechanics_readiness using their canonical
generators first. This report never overrides gate inputs or verdicts.
"""
from __future__ import annotations

import json
import argparse
from collections import Counter
from pathlib import Path

from data_paths import REPO_ROOT
from digimon_gym.agents.card_admissibility import CardAdmissibility


def build(root: Path = REPO_ROOT, *, runtime: bool = False) -> dict:
    fixture = json.loads((root / "qa/archetype-qa/dynasmon-tesero-120647.json").read_text(encoding="utf-8"))
    exact = Counter(fixture["main"]) + Counter(fixture["eggs"])
    assert sum(fixture["main"].values()) == 50
    assert sum(fixture["eggs"].values()) == 4
    assert len(exact) == 22
    library = json.loads((root / "data/deck_library.json").read_text(encoding="utf-8"))
    lists = library["archetypes"]["Dynasmon"]["decklists"]
    artifact = root / "data/card_admissibility.json"
    artifact_cards = json.loads(artifact.read_text(encoding="utf-8"))["cards"]
    admission = CardAdmissibility.from_artifact(artifact)
    ledgers = {name: json.loads((root / "qa/qa-reports" / name).read_text(encoding="utf-8"))["cards"]
               for name in ("validated_cards.json", "validated_cards_dsl.json")}
    mechanics = json.loads((root / "data/mechanics_readiness.json").read_text(encoding="utf-8"))
    cards = json.loads((root / "data/cards.json").read_text(encoding="utf-8"))
    # Production metadata is keyed by printed card ID.
    present = set(cards) if isinstance(cards, dict) else {c["card_id"] for c in cards}

    def row(label: str, url: str, counts: Counter) -> dict:
        failures = {}
        for card in sorted(counts):
            reasons = admission.reasons(card)
            if card not in present:
                reasons = ["missing_production_data", *reasons]
            if reasons:
                spec = root / "code/digimon-engine/cards" / card.split("-")[0].lower() / f"{card}.yaml"
                verdicts = {name: {key: ledger[card][key] for key in ("status", "gap_kind", "notes", "report") if key in ledger[card]}
                            for name, ledger in ledgers.items() if card in ledger}
                failures[card] = {"reasons": reasons, "constructs": admission.constructs_missing(card),
                                  "production_yaml_exists": spec.exists(),
                                  "has_card_test": artifact_cards.get(card, {}).get("has_card_test", False),
                                  "ledger_verdicts": verdicts}
        return {"list": label, "source_url": url, "card_count": sum(counts.values()),
                "unique_ids": len(counts), "cards": dict(sorted(counts.items())),
                "same_counts_as_primary": counts == exact,
                "admitted": mechanics["ready"] and not failures, "failures": failures}

    rows = [row("digilab_120647", fixture["source_url"], exact)]
    rows += [row(d["deck_id"], d["source_url"], Counter(json.loads(d["decklist"]))) for d in lists]
    for measured, source in zip(rows[1:], lists):
        measured.update({key: source.get(key) for key in ("event_date", "format", "player_name", "source")})
    result = {"mechanics_ready": mechanics["ready"], "mechanics_blocking": mechanics["blocking"],
            "primary": rows[0], "repository_lists": rows[1:],
            "repository_list_count": len(lists), "repository_admitted": sum(r["admitted"] for r in rows[1:]),
            "runtime_verified": False}
    current = [r for r in rows[1:] if r["format"] == "EX13"]
    result["ex13_list_count"] = len(current)
    result["ex13_unique_ids"] = len({card for row in current for card in row["cards"]})
    result["ex13_admitted"] = sum(r["admitted"] for r in current)
    if runtime:
        # Use the real loader with its default gates and compiled registry.
        # No injected implemented IDs, readiness, or admissibility objects.
        from digimon_gym.agents.gauntlet import (
            MetaGauntlet, load_playable_card_ids, require_mechanics_ready,
            validate_admissible_deck, validate_implemented_deck, stable_deck_id,
            NotAdmissibleDeckError, UnimplementedDeckError,
        )
        require_mechanics_ready()
        playable = load_playable_card_ids()
        for measured in rows:
            ids = list(Counter(measured["cards"]).elements())
            for card, failure in measured["failures"].items():
                failure["runtime_playable"] = card in playable
            try:
                validate_implemented_deck(ids, playable, label=measured["list"])
                validate_admissible_deck(ids, label=measured["list"])
                measured["runtime_admitted"] = True
            except (NotAdmissibleDeckError, UnimplementedDeckError) as error:
                measured["runtime_admitted"] = False
                measured["runtime_rejection"] = str(error)
            assert measured["runtime_admitted"] == measured["admitted"], measured
        gauntlet = MetaGauntlet(allowed_archetypes={"Dynasmon"})
        gauntlet.load(str(root / "data/deck_library.json"))
        loaded = gauntlet.as_generalist_pool()
        entries = loaded.archetypes.get("Dynasmon", [])
        expected = {stable_deck_id(list(Counter(r["cards"]).elements())) for r in rows[1:] if r["admitted"]}
        assert {d.deck_id for d in entries} == expected
        result["gauntlet_source_deck_ids"] = sorted(d.source_deck_id for d in entries)
        from digimon_engine import RustHeadlessGame, ACTION_SPACE_SIZE, get_observation_layout
        profile = "standard_lite_v2"
        tensor_size = get_observation_layout(profile)["tensor_size"]
        for measured in rows:
            if not measured["runtime_admitted"]:
                continue
            ids = list(Counter(measured["cards"]).elements())
            game = RustHeadlessGame(ids, ids, seed=120647, observation_profile=profile)
            assert len(game.get_board_tensor(1)) == tensor_size, measured["list"]
            assert len(game.get_action_mask()) == ACTION_SPACE_SIZE, measured["list"]
            assert any(game.get_action_mask()), measured["list"]
            measured["runtime_game_loaded"] = True
        result["runtime_game_loaded"] = rows[0].get("runtime_game_loaded", False)
        result["runtime_repository_games_loaded"] = sum(r.get("runtime_game_loaded", False) for r in rows[1:])
        result["runtime_contract"] = {"profile": profile, "tensor_size": tensor_size, "action_space_size": ACTION_SPACE_SIZE}
        result["runtime_verified"] = True
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runtime", action="store_true", help="Also validate through a fresh installed Rust wheel and the real gauntlet loader")
    result = build(runtime=parser.parse_args().runtime)
    destination = REPO_ROOT / "qa/archetype-qa/dynasmon-admission.json"
    destination.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(f"Primary admitted: {result['primary']['admitted']}; repository lists: "
          f"{result['repository_admitted']}/{result['repository_list_count']}; EX13 lists: "
          f"{result['ex13_admitted']}/{result['ex13_list_count']}; wrote {destination}")
