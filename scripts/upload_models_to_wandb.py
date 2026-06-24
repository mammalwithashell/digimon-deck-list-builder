#!/usr/bin/env python3
"""
Attach the starter-league specialist checkpoints (final.zip + round checkpoints)
to their existing W&B runs as `model` artifacts.

WHY THIS RUNS ON THE TRAINING BOX:
  The .zip model files live only on the training host (e.g. /app/models/
  specialists/st-<n>/r<round>/). W&B logged the *metrics* online but never
  uploaded the *weights*. This script reads each finished run's own config to
  find where its models_dir/run_name lives on THIS machine, then resumes the
  run and logs the checkpoints as an artifact.

USAGE (on the box, where wandb is already authenticated):
  # If the models are inside the stopped training container, copy them out first:
  #   docker cp <container>:/app/models ./models
  # then point MODELS_ROOT at it (optional; the script also trusts each run's
  # config models_dir as-is).
  export WANDB_API_KEY=...                 # already set from training, usually
  python3 upload_models_to_wandb.py \
      --entity james-peters97-mammal --project Digimon-rl \
      [--models-root /path/to/models] [--dry-run]

IDEMPOTENT: skips a run if it already has an artifact named "<run>-model".
"""
import argparse
import os
import sys
from pathlib import Path

import wandb


def round_dir_for(config: dict, models_root: str | None) -> Path | None:
    """Best-effort: where does THIS run's checkpoint dir live on disk?

    Layout observed in the starter-league curriculum:
        {models_dir}/{run_name}/final.zip   e.g. /app/models/specialists/st-6/r8/final.zip
    `models_dir` already includes the specialist (.../specialists/st-6); the
    per-round subdir is `run_name` (e.g. "r8").
    """
    models_dir = config.get("models_dir")
    run_name = config.get("run_name")
    if not models_dir or not run_name:
        return None
    p = Path(models_dir) / str(run_name)
    if models_root:
        # Re-root /app/models/... under a local copy (e.g. after `docker cp`).
        suffix = Path(models_dir).relative_to("/app/models") if str(models_dir).startswith("/app/models") else Path(models_dir).name
        p = Path(models_root) / suffix / str(run_name)
    return p


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--entity", required=True)
    ap.add_argument("--project", required=True)
    ap.add_argument("--models-root", default=os.environ.get("MODELS_ROOT"),
                    help="Re-root /app/models under this local dir (e.g. after docker cp).")
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()

    api = wandb.Api()
    runs = list(api.runs(f"{args.entity}/{args.project}"))
    print(f"{len(runs)} runs in {args.entity}/{args.project}\n")

    uploaded, skipped, missing = 0, 0, []
    for r in runs:
        cfg = {k: v for k, v in r.config.items()}
        rd = round_dir_for(cfg, args.models_root)
        if rd is None:
            print(f"[skip] {r.name}: no models_dir/run_name in config")
            continue
        zips = sorted(rd.glob("*.zip")) if rd.is_dir() else []
        if not zips:
            print(f"[MISS] {r.name}: no .zip under {rd}")
            missing.append((r.name, str(rd)))
            continue

        art_name = f"{r.name}-model"
        # idempotency: skip if this run already logged that artifact
        already = any(a.name.split(":")[0] == art_name for a in r.logged_artifacts())
        if already:
            print(f"[have] {r.name}: {art_name} already attached ({len(zips)} zip on disk)")
            skipped += 1
            continue

        final = next((z for z in zips if z.name == "final.zip"), None)
        ckpts = [z for z in zips if z.name != "final.zip"]
        print(f"[push] {r.name}: {len(zips)} zip from {rd} "
              f"(final={'yes' if final else 'NO'}, checkpoints={len(ckpts)})")
        if args.dry_run:
            continue

        run = wandb.init(entity=args.entity, project=args.project, id=r.id,
                         resume="must", reinit=True)
        art = wandb.Artifact(
            name=art_name, type="model",
            description=f"Starter-league specialist {r.name}: final + round checkpoints",
            metadata={"deck": cfg.get("allowed_archetypes"),
                      "round": cfg.get("run_name"),
                      "timesteps": cfg.get("timesteps"),
                      "init_from": cfg.get("init_from"),
                      "anchored_panel_mean": r.summary.get("pilot/anchored/panel_mean")},
        )
        for z in zips:
            art.add_file(str(z), name=z.name)
        aliases = ["latest", "round-final"] if final else ["latest"]
        run.log_artifact(art, aliases=aliases)
        art.wait()
        run.finish()
        uploaded += 1

    print(f"\ndone: {uploaded} uploaded, {skipped} already-present, {len(missing)} missing files")
    for name, path in missing:
        print(f"  missing: {name} -> {path}")
    return 0 if not missing else 1


if __name__ == "__main__":
    sys.exit(main())
