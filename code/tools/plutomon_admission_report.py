"""Measure the exact NewGuy list using canonical gate artifacts, optionally the real runtime.

Regenerate lowering_coverage, card_admissibility and mechanics_readiness first.
This tool does not override gate decisions or imply DCGO verification.
"""
from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import random
import subprocess
import sys
import tempfile

from data_paths import REPO_ROOT
from digimon_gym.agents.card_admissibility import CardAdmissibility


def build(*, runtime: bool = False) -> dict:
    root = REPO_ROOT
    fixture = json.loads((root / "qa/archetype-qa/ts-plutomon-newguy-131677.json").read_text())
    counts = Counter(fixture["main"]) + Counter(fixture["eggs"])
    assert sum(fixture["main"].values()) == 50
    assert sum(fixture["eggs"].values()) == 4
    assert len(counts) == 19
    admission = CardAdmissibility.from_artifact(root / "data/card_admissibility.json")
    mechanics = json.loads((root / "data/mechanics_readiness.json").read_text())
    failures = {cid: {"reasons": admission.reasons(cid), "constructs": admission.constructs_missing(cid)}
                for cid in sorted(counts) if admission.reasons(cid)}
    result = {"source_url": fixture["source_url"], "cards": dict(sorted(counts.items())),
              "revision": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip(),
              "mechanics_test_sha256": hashlib.sha256((root / "code/digimon-engine/tests/dsl/plutomon_admission_constructs.rs").read_bytes()).hexdigest(),
              "main_count": 50, "egg_count": 4, "unique_ids": 19,
              "mechanics_ready": mechanics["ready"], "mechanics_blocking": mechanics["blocking"],
              "admitted": mechanics["ready"] and not failures, "failures": failures,
              "runtime_verified": False, "dcgo_verified": False}
    if runtime:
        import digimon_engine
        from digimon_engine import RustHeadlessGame, ACTION_SPACE_SIZE, get_observation_layout
        from digimon_gym.agents.gauntlet import MetaGauntlet, stable_deck_id

        ids = list(counts.elements())
        library = {"archetypes": {"TS Plutomon": {"decklists": [{
            "deck_id": "digilab_131677", "source": "digilab", "source_url": fixture["source_url"],
            "decklist": json.dumps(ids)}]}}}
        # Exact-list input only; all loader gates, registry and database remain defaults.
        with tempfile.TemporaryDirectory(prefix="plutomon-exact-") as tmp:
            path = Path(tmp) / "library.json"
            path.write_text(json.dumps(library), encoding="utf-8")
            gauntlet = MetaGauntlet()
            gauntlet.load(str(path))
        entries = [entry for arch in gauntlet.archetypes.values() for entry in arch.decks]
        assert len(entries) == 1, result
        assert Counter(entries[0].card_ids) == counts
        assert entries[0].deck_id == stable_deck_id(ids)
        assert result["admitted"], result
        result["gauntlet_source_deck_id"] = entries[0].source_deck_id
        profile = "standard_lite_v2"
        size = get_observation_layout(profile)["tensor_size"]
        smokes = []
        for seed in [131677, 131678]:
            game = RustHeadlessGame(ids, ids, seed=seed, observation_profile=profile)
            rng = random.Random(seed)
            steps = 0
            actions = []
            while steps < 100 and not game.is_game_over:
                assert len(game.get_board_tensor(1)) == size
                mask = game.get_action_mask()
                assert len(mask) == ACTION_SPACE_SIZE
                legal = [i for i, value in enumerate(mask) if value and i != 93]
                assert legal, "no non-concede legal action"
                action = rng.choice(legal)
                actions.append(action)
                try:
                    game.step(action)
                except BaseException:
                    # PyO3 panic exceptions inherit BaseException. Keep the exact
                    # failing trace, then propagate it; never turn crashes into draws.
                    print(json.dumps({"failed_seed": seed, "actions": actions}), file=sys.stderr)
                    raise
                steps += 1
            smokes.append({"seed": seed, "legal_steps": steps, "game_over": game.is_game_over})
        result.update(runtime_verified=True, smokes=smokes,
                      binding_file=digimon_engine.__file__,
                      runtime_contract={"profile": profile, "tensor_size": size,
                                        "action_space_size": ACTION_SPACE_SIZE})
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runtime", action="store_true")
    parser.add_argument("--out", type=Path, default=REPO_ROOT / "qa/archetype-qa/ts-plutomon-admission.json")
    args = parser.parse_args()
    result = build(runtime=args.runtime)
    args.out.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(result, indent=2))
