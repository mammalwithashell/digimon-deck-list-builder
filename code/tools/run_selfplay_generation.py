"""Self-play generation orchestrator (add-determinized-search task 3.5).

Runs one or more AlphaZero generations end-to-end:

    N parallel `digimon-engine-cli selfplay` processes
        -> pool this gen's shards + a sliding window of past gens
        -> azero_trainer.train() (warm-started from the previous gen's .zip)
        -> export_onnx.py (via azero_trainer's --export-onnx hop)
        -> anchored_eval_cli.py (optional; the promotion signal, rule 30)
        -> gen<G>/summary.json + run-level state.json

Layout under --run-dir:
    state.json                 latest completed generation + paths
    gen0/shards/proc<i>/       selfplay-shards-v1 dirs (one per process)
    gen0/model.zip             trained SB3 checkpoint (or the seed model for gen 0)
    gen0/model.onnx            re-exported policy+value ONNX (driver input for gen1)
    gen0/anchored_eval.txt     raw anchored_eval_cli.py stdout
    gen0/summary.json          per-generation stats + timings

Gen 0 is special: there is no prior generation to warm-start the SELF-PLAY
from, so gen 0's shards come from `--seed-model` directly (or `--uniform`
for a cold start); the trainer still warm-starts from `--seed-model`.
Every subsequent generation self-plays with the previous gen's `model.onnx`
and trains from the previous gen's `model.zip`.

Usage:
    python code/tools/run_selfplay_generation.py \
        --run-dir runs/selfplay/run1 \
        --seed-model models/starter_pool_single_v1/final.zip \
        --decks training_jobs/selfplay_decks.json \
        --cli-bin target/release/digimon-engine-cli \
        --generations 3 --games 200 --procs 8 --sims 320 --worlds 4

Resume: re-run the same command — state.json tells it which generation to
start from; a partially-written generation directory is overwritten (a
crashed self-play stage leaves no usable shards, so re-running it is safe).
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]


def _cli_bin_default() -> str:
    suffix = ".exe" if sys.platform == "win32" else ""
    for profile in ("release", "debug"):
        candidate = REPO_ROOT / "target" / profile / f"digimon-engine-cli{suffix}"
        if candidate.exists():
            return str(candidate)
    return str(REPO_ROOT / "target" / "release" / f"digimon-engine-cli{suffix}")


def _split_games(total: int, procs: int) -> list[int]:
    """Divide `total` games across `procs` as evenly as possible."""
    base, extra = divmod(total, procs)
    return [base + (1 if i < extra else 0) for i in range(procs)]


def load_state(run_dir: Path) -> dict:
    state_path = run_dir / "state.json"
    if state_path.exists():
        return json.loads(state_path.read_text())
    return {"next_gen": 0}


def save_state(run_dir: Path, state: dict) -> None:
    (run_dir / "state.json").write_text(json.dumps(state, indent=2))


def run_selfplay_stage(
    cli_bin: str,
    gen_dir: Path,
    model_onnx: str | None,
    uniform: bool,
    decks: Path,
    games: int,
    procs: int,
    sims: int,
    worlds: int,
    mode: str,
    seed_base: int,
    temperature_decisions: int,
    decision_cap: int,
    shard_rows: int,
    cards_json: Path | None,
    pool: str,
) -> list[Path]:
    """Launch `procs` parallel selfplay processes; return their out dirs."""
    shards_root = gen_dir / "shards"
    shards_root.mkdir(parents=True, exist_ok=True)
    per_proc_games = _split_games(games, procs)

    procs_running: list[tuple[subprocess.Popen, Path, Path]] = []
    for i, n_games in enumerate(per_proc_games):
        if n_games == 0:
            continue
        out_dir = shards_root / f"proc{i}"
        log_path = gen_dir / f"selfplay_proc{i}.log"
        cmd = [
            cli_bin,
            "--pool",
            pool,
        ]
        if cards_json is not None:
            cmd += ["--cards-json", str(cards_json)]
        cmd += [
            "selfplay",
            "--decks",
            str(decks),
            "--games",
            str(n_games),
            "--sims",
            str(sims),
            "--worlds",
            str(worlds),
            "--mode",
            mode,
            "--seed",
            str(seed_base + i),
            "--out",
            str(out_dir),
            "--temperature-decisions",
            str(temperature_decisions),
            "--decision-cap",
            str(decision_cap),
            "--shard-rows",
            str(shard_rows),
        ]
        if uniform:
            cmd.append("--uniform")
        else:
            cmd += ["--model", str(model_onnx)]

        log_f = open(log_path, "w", encoding="utf-8")
        proc = subprocess.Popen(cmd, stdout=log_f, stderr=subprocess.STDOUT)
        procs_running.append((proc, out_dir, log_path))
        print(f"  [proc{i}] {n_games} games, seed {seed_base + i} -> {out_dir}", flush=True)

    failures = []
    out_dirs = []
    for proc, out_dir, log_path in procs_running:
        rc = proc.wait()
        if rc != 0:
            failures.append((log_path, rc))
        else:
            out_dirs.append(out_dir)
    if failures:
        details = "; ".join(f"{p} (exit {rc})" for p, rc in failures)
        raise RuntimeError(f"selfplay process(es) failed: {details}")
    return out_dirs


def collect_window_shard_dirs(run_dir: Path, gen: int, window: int) -> list[Path]:
    """This gen's proc dirs + the same from up to `window - 1` prior gens."""
    dirs: list[Path] = []
    for g in range(max(0, gen - window + 1), gen + 1):
        shards_root = run_dir / f"gen{g}" / "shards"
        if not shards_root.exists():
            continue
        dirs.extend(sorted(shards_root.glob("proc*")))
    return dirs


ANCHOR_LINE = re.compile(r"vs\s+(\S+)\s+\d+-\d+-\d+\s+WR=\s*([\d.]+)%")


def run_anchored_eval(
    candidate_zip: Path,
    out_path: Path,
    n: int,
    deck_pool_snapshot: Path | None,
    seed: int,
    device: str,
) -> dict:
    cmd = [
        sys.executable,
        str(REPO_ROOT / "code" / "tools" / "anchored_eval_cli.py"),
        "--candidate",
        str(candidate_zip),
        "--n",
        str(n),
        "--seed",
        str(seed),
        "--device",
        device,
    ]
    if deck_pool_snapshot is not None:
        cmd += ["--deck-pool-snapshot", str(deck_pool_snapshot)]
    env = _child_env()
    result = subprocess.run(cmd, capture_output=True, text=True, env=env)
    out_path.write_text(result.stdout + "\n" + result.stderr)
    if result.returncode != 0:
        print(f"  WARNING: anchored eval failed (exit {result.returncode}); see {out_path}")
        return {"ok": False}
    anchors = {name: float(wr) for name, wr in ANCHOR_LINE.findall(result.stdout)}
    return {"ok": True, "win_rates": anchors}


def _child_env() -> dict:
    import os

    env = dict(os.environ)
    code_dir = str(REPO_ROOT / "code")
    env["PYTHONPATH"] = code_dir + (
        (":" if sys.platform != "win32" else ";") + env["PYTHONPATH"] if env.get("PYTHONPATH") else ""
    )
    return env


def run_one_generation(args: argparse.Namespace, gen: int, prev: dict | None) -> dict:
    gen_dir = Path(args.run_dir) / f"gen{gen}"
    if gen_dir.exists():
        shutil.rmtree(gen_dir)
    gen_dir.mkdir(parents=True)

    t0 = time.time()
    is_gen0 = prev is None
    seed_model = Path(args.seed_model)
    model_onnx_for_selfplay = None if (is_gen0 and args.uniform) else (
        str(resolve_seed_model_onnx(args.seed_model, args.seed_model_onnx)) if is_gen0 else prev["model_onnx"]
    )
    warm_start_zip = str(seed_model) if is_gen0 else prev["model_zip"]

    print(f"=== generation {gen} ===", flush=True)
    print(f"self-play: {args.games} games / {args.procs} procs, warm-start selfplay model = "
          f"{'uniform' if (is_gen0 and args.uniform) else model_onnx_for_selfplay}", flush=True)
    proc_dirs = run_selfplay_stage(
        cli_bin=args.cli_bin,
        gen_dir=gen_dir,
        model_onnx=model_onnx_for_selfplay,
        uniform=is_gen0 and args.uniform,
        decks=Path(args.decks),
        games=args.games,
        procs=args.procs,
        sims=args.sims,
        worlds=args.worlds,
        mode=args.mode,
        seed_base=args.seed + gen * 100_000,
        temperature_decisions=args.temperature_decisions,
        decision_cap=args.decision_cap,
        shard_rows=args.shard_rows,
        cards_json=Path(args.cards_json) if args.cards_json else None,
        pool=args.pool,
    )
    t_selfplay = time.time() - t0

    window_dirs = collect_window_shard_dirs(Path(args.run_dir), gen, args.window)
    print(f"training: warm-start {warm_start_zip}, window = {len(window_dirs)} shard dir(s)", flush=True)

    sys.path.insert(0, str(REPO_ROOT / "code"))
    from digimon_gym.agents.azero_trainer import train as azero_train  # noqa: E402

    model_zip = gen_dir / "model.zip"
    model_onnx = gen_dir / "model.onnx"
    t1 = time.time()
    train_summary = azero_train(
        init_from=Path(warm_start_zip),
        shard_dirs=window_dirs,
        out=model_zip,
        epochs=args.epochs,
        batch_size=args.batch_size,
        lr=args.lr,
        value_coef=args.value_coef,
        device=args.train_device,
        seed=args.seed + gen,
        export_onnx=model_onnx,
    )
    t_train = time.time() - t1

    eval_result = {"ok": False, "skipped": True}
    t_eval = 0.0
    if not args.skip_eval:
        t2 = time.time()
        eval_result = run_anchored_eval(
            candidate_zip=model_zip,
            out_path=gen_dir / "anchored_eval.txt",
            n=args.eval_n,
            deck_pool_snapshot=Path(args.deck_pool_snapshot) if args.deck_pool_snapshot else None,
            seed=args.seed,
            device=args.train_device,
        )
        t_eval = time.time() - t2
        print(f"anchored eval: {eval_result}", flush=True)

    summary = {
        "generation": gen,
        "proc_dirs": [str(p) for p in proc_dirs],
        "window_dirs": [str(p) for p in window_dirs],
        "train": train_summary,
        "anchored_eval": eval_result,
        "timings_sec": {
            "selfplay": t_selfplay,
            "train": t_train,
            "eval": t_eval,
            "total": time.time() - t0,
        },
    }
    (gen_dir / "summary.json").write_text(json.dumps(summary, indent=2))
    print(
        f"generation {gen} done in {summary['timings_sec']['total']:.0f}s "
        f"(selfplay {t_selfplay:.0f}s, train {t_train:.0f}s, eval {t_eval:.0f}s)",
        flush=True,
    )
    return {"model_zip": str(model_zip), "model_onnx": str(model_onnx), "summary": summary}


def resolve_seed_model_onnx(seed_model: str, explicit: str | None) -> Path:
    """Gen 0's self-play evaluator export. Prefers `--seed-model-onnx`;
    falls back to a same-stem `.onnx` sibling of `--seed-model` (NOT
    guaranteed to match — a checkpoint named `final.zip` does not imply
    `final.onnx` exists, since export filenames are chosen independently).
    """
    if explicit is not None:
        p = Path(explicit)
        if not p.exists():
            raise FileNotFoundError(f"--seed-model-onnx {p} does not exist")
        return p
    candidate = Path(seed_model).with_suffix(".onnx")
    if candidate.exists():
        return candidate
    raise FileNotFoundError(
        f"no --seed-model-onnx given and no sibling {candidate} exists — "
        f"pass --seed-model-onnx explicitly (export one with "
        f"code/tools/export_onnx.py --type mlp), or pass --uniform for a cold start"
    )


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    p.add_argument("--run-dir", required=True, help="root directory for this self-play run")
    p.add_argument("--seed-model", required=True, help="SB3 .zip to warm-start generation 0's TRAINING from")
    p.add_argument("--seed-model-onnx", default=None, help="policy+value ONNX for generation 0's SELF-PLAY (default: a same-stem .onnx sibling of --seed-model)")
    p.add_argument("--uniform", action="store_true", help="gen 0 self-plays with the uniform stub evaluator instead of an ONNX export")
    p.add_argument("--decks", required=True, help="deck-pool JSON for the selfplay driver")
    p.add_argument("--cli-bin", default=None, help="path to digimon-engine-cli (default: auto-detect target/{release,debug})")
    p.add_argument("--cards-json", default=None)
    p.add_argument("--pool", default="implemented")

    p.add_argument("--generations", type=int, default=1, help="how many generations to run this invocation")
    p.add_argument("--games", type=int, default=200, help="self-play games PER generation")
    p.add_argument("--procs", type=int, default=8, help="parallel selfplay processes")
    p.add_argument("--sims", type=int, default=320)
    p.add_argument("--worlds", type=int, default=4)
    p.add_argument("--mode", default="pimc")
    p.add_argument("--temperature-decisions", type=int, default=16)
    p.add_argument("--decision-cap", type=int, default=600)
    p.add_argument("--shard-rows", type=int, default=4096)

    p.add_argument("--window", type=int, default=3, help="sliding-window generations of shards for training")
    p.add_argument("--epochs", type=int, default=2)
    p.add_argument("--batch-size", type=int, default=512)
    p.add_argument("--lr", type=float, default=1e-4)
    p.add_argument("--value-coef", type=float, default=1.0)
    p.add_argument("--train-device", default="auto")

    p.add_argument("--skip-eval", action="store_true")
    p.add_argument("--eval-n", type=int, default=300)
    p.add_argument("--deck-pool-snapshot", default=None)

    p.add_argument("--seed", type=int, default=0)
    return p


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    if args.cli_bin is None:
        args.cli_bin = _cli_bin_default()
    if not Path(args.cli_bin).exists():
        print(f"error: cli binary not found at {args.cli_bin} — build it first: "
              f"cargo build --release -p digimon-engine-cli", file=sys.stderr)
        return 2

    run_dir = Path(args.run_dir)
    run_dir.mkdir(parents=True, exist_ok=True)
    state = load_state(run_dir)
    start_gen = state["next_gen"]
    prev = state.get("prev")

    for gen in range(start_gen, start_gen + args.generations):
        result = run_one_generation(args, gen, prev)
        prev = {"model_zip": result["model_zip"], "model_onnx": result["model_onnx"]}
        state = {"next_gen": gen + 1, "prev": prev}
        save_state(run_dir, state)

    print(f"\ndone: generations {start_gen}..{start_gen + args.generations - 1} in {run_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
