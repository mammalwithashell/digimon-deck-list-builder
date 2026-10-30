"""Record every closed-search pool probe, including failures and truncations."""
from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def run(args):
    args.out.mkdir(parents=True, exist_ok=False)
    rows = json.loads(args.decks.read_bytes())
    if not rows:
        raise ValueError("an empty cohort cannot qualify closed search")
    # Preserve source ordering/seeds; repeated sampler slots aren't new probes.
    distinct = {}
    for row in rows:
        distinct.setdefault(tuple(sorted(row["cards"])), row)
    identity = {"engine": sha(args.cli), "decks": sha(args.decks),
                "native_libraries": {p.name: sha(p) for p in sorted(args.cli.parent.glob("*.dll"))},
                "workers": args.workers,
                "prior": sha(args.prior), "script": sha(__file__),
                "profile": args.profile, "simulations": args.sims,
                "worlds": args.worlds, "seed": args.seed,
                "decision_cap": args.decision_cap}
    (args.out / "identity.json").write_text(json.dumps(identity, indent=2), encoding="utf-8")

    def probe(item):
        index, row = item
        dest = args.out / f"case-{index:02d}"
        dest.mkdir()
        deck = dest / "deck.json"
        deck.write_text(json.dumps([row]), encoding="utf-8")
        command = [str(args.cli), "--cards-json", str(ROOT / "data/cards.json"),
                   "selfplay", "--uniform", "--decks", str(deck),
                   "--information-regime", "closed_decklist", "--belief-prior", str(args.prior),
                   "--tensor-profile", args.profile, "--games", "1", "--sims", str(args.sims),
                   "--worlds", str(args.worlds), "--eval-batch-size", str(args.worlds),
                   "--decision-cap", str(args.decision_cap), "--seed", str(args.seed + index),
                   "--out", str(dest / "game")]
        (dest / "command.json").write_text(json.dumps(command, indent=2), encoding="utf-8")
        started = time.monotonic()
        with (dest / "run.log").open("w", encoding="utf-8") as log:
            result = subprocess.run(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT)
        manifest = dest / "game/manifest.json"
        record = {"index": index, "archetype": row.get("archetype", row.get("name")),
                  "deck_id": row.get("deck_id"), "returncode": result.returncode,
                  "seconds": time.monotonic() - started, "status": "failed"}
        if result.returncode == 0 and manifest.exists():
            data = json.loads(manifest.read_bytes())
            record.update(games=data["games"], truncated=data["truncated_games"],
                          status="completed" if data["games"] == 1 and data["truncated_games"] == 0 else "truncated")
        else:
            record["error"] = (dest / "run.log").read_text(encoding="utf-8").splitlines()[-1:]
        print(json.dumps(record), flush=True)
        return record

    with ThreadPoolExecutor(max_workers=args.workers) as workers:
        results = list(workers.map(probe, enumerate(distinct.values())))
    report = {"identity": identity, "cases": results,
              "completed": sum(r["status"] == "completed" for r in results),
              "failed": sum(r["status"] == "failed" for r in results),
              "truncated": sum(r["status"] == "truncated" for r in results),
              "engine_unchanged": sha(args.cli) == identity["engine"] and
                  {p.name: sha(p) for p in sorted(args.cli.parent.glob("*.dll"))} == identity["native_libraries"],
              "probe_readiness_only": True}
    (args.out / "report.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    return report["completed"] == len(results) and report["engine_unchanged"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ("cli", "decks", "prior", "out"):
        parser.add_argument("--" + key, type=Path, required=True)
    parser.add_argument("--profile", default="standard_lite_v2")
    parser.add_argument("--sims", type=int, default=4)
    parser.add_argument("--worlds", type=int, default=2)
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--decision-cap", type=int, default=1200)
    parser.add_argument("--seed", type=int, default=20261029)
    args = parser.parse_args()
    for key in ("cli", "decks", "prior", "out"):
        setattr(args, key, getattr(args, key).resolve())
    if min(args.workers, args.sims, args.worlds, args.decision_cap) < 1:
        parser.error("budgets must be positive")
    raise SystemExit(0 if run(args) else 1)


if __name__ == "__main__":
    main()
