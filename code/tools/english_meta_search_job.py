"""Prepare and bootstrap a reproducible English EX13 search pilot.

The training/check pools contain original tournament lists, never repaired
substitutes. The bootstrap is explicitly greedy imitation from a fresh,
current-contract network when no compatible checkpoint is available.
"""
from __future__ import annotations

import argparse
from collections import Counter
from datetime import date, datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
ARCHETYPES = ["Examon", "TS Jupitermon", "Glowing Dawn", "TS Mervamon",
              "Chronicle", "Toho Braves", "Three Musketeers", "Chronomon"]
PROFILE = "standard_lite_v2"


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, ensure_ascii=True), encoding="utf-8")


def event_date(raw):
    for fmt in ("%Y-%m-%d", "%m/%d/%Y"):
        try:
            return datetime.strptime(str(raw), fmt).date()
        except ValueError:
            pass
    return date.min


def prepare(run_dir: Path, cli_bin: Path):
    import digimon_engine
    from digimon_gym.agents.gauntlet import (
        DeckEntry, GeneralistDeckPool, require_mechanics_ready, validate_admissible_deck, stable_deck_id,
    )
    require_mechanics_ready()
    registered = set(digimon_engine.load_implemented_card_ids())
    native = set(json.loads(subprocess.check_output(
        [str(cli_bin), "--cards-json", str(ROOT / "data/cards.json"), "pool"], text=True)))
    database = json.loads((ROOT / "data/cards.json").read_text(encoding="utf-8"))
    data_ids = {row["card_id"] for row in database} if isinstance(database, list) else set(database)
    if native != registered & data_ids:
        raise ValueError("Rust CLI and Python wheel registries differ; rebuild matching binaries")
    library_path = ROOT / "data/deck_library.json"
    library = json.loads(library_path.read_text(encoding="utf-8"))
    selected, heldout, report = {}, {}, {}
    for arch in ARCHETYPES:
        eligible, rejected, seen = [], Counter(), set()
        rows = library["archetypes"][arch]["decklists"]
        for entry in sorted(rows, key=lambda r: (event_date(r.get("event_date")), bool(r.get("is_top_cut"))), reverse=True):
            if entry.get("format") != "EX13" or not date(2026, 10, 1) <= event_date(entry.get("event_date")) <= date(2026, 10, 23):
                rejected["outside English EX13 October window"] += 1
                continue
            cards = entry["decklist"]
            cards = json.loads(cards) if isinstance(cards, str) else cards
            key = tuple(sorted(Counter(cards).items()))
            if key in seen:
                rejected["duplicate card counts"] += 1
                continue
            if not set(cards) <= native:
                rejected["unplayable"] += 1
                continue
            try:
                validate_admissible_deck(cards, label=entry["deck_id"])
            except ValueError:
                rejected["card admission"] += 1
                continue
            legality = digimon_engine.validate_deck(cards)
            if not legality.is_valid:
                rejected["English deck legality"] += 1
                continue
            seen.add(key)
            eligible.append((DeckEntry(deck_id=stable_deck_id(cards), archetype_name=arch,
                card_ids=cards, source=entry.get("source", ""), source_deck_id=entry["deck_id"],
                source_url=entry.get("source_url", "")), entry))
        if len(eligible) < 3:
            raise ValueError(f"{arch}: need 3 distinct admitted legal lists, found {len(eligible)}; {dict(rejected)}")
        selected[arch] = [d for d, _ in eligible[:2]]
        heldout[arch] = [eligible[2][0]]
        report[arch] = {"eligible_distinct_lists": len(eligible), "rejected": dict(rejected),
            "training": [{k:v for k,v in e.items() if k != "decklist"} for _,e in eligible[:2]],
            "heldout": {k:v for k,v in eligible[2][1].items() if k != "decklist"}}
    train_pool = GeneralistDeckPool(selected)
    train_pool.write_snapshot(run_dir / "train_snapshot.json")
    GeneralistDeckPool(heldout).write_snapshot(run_dir / "heldout_snapshot.json")
    write_json(run_dir / "decks.json", [{"name":f"{arch}:{d.deck_id}","cards":d.card_ids}
        for arch, decks in selected.items() for d in decks])
    report = {"scope":"English EX13-labelled October tournament lists; admitted supported subset",
        "sampling":"uniform archetype (2 original lists each)","archetypes":report,
        "missing_meta":"Ulforce excluded by admission; no whole-meta readiness claim",
        "source_meta":"https://digimon.digilab.cards/meta", "as_of":"2026-10-23",
        "library_sha256":hashlib.sha256(library_path.read_bytes()).hexdigest(),
        "artifact_hashes":{n:hashlib.sha256((ROOT / "data" / n).read_bytes()).hexdigest()
            for n in ["cards.json","card_admissibility.json","mechanics_readiness.json","deck_formats.json"]}}
    write_json(run_dir / "pool_report.json", report)
    extensions = list(Path(digimon_engine.__file__).parent.glob("*.pyd"))
    write_json(run_dir / "provenance.json", {
        "git_head":subprocess.check_output(["git","rev-parse","HEAD"],cwd=ROOT,text=True).strip(),
        "working_diff_sha256":hashlib.sha256(subprocess.check_output(["git","diff","--binary"],cwd=ROOT)).hexdigest(),
        "cli_path":str(cli_bin.resolve()),"cli_sha256":hashlib.sha256(cli_bin.read_bytes()).hexdigest(),
        "python":sys.executable,"engine_extensions":{str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in extensions},
        "source_files":{str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in [
            ROOT / "code/tools/english_meta_search_job.py",ROOT / "code/tools/run_selfplay_generation.py",
            ROOT / "code/digimon_gym/agents/azero_trainer.py"]},
    })
    print(f"Prepared {train_pool.archetype_count} archetypes, {train_pool.deck_count} training lists, 8 held-out lists",flush=True)


def collect_bootstrap(run_dir: Path, games: int, seed: int):
    from digimon_engine import RustHeadlessGame
    from model_contract import current_contract
    decks = json.loads((run_dir / "decks.json").read_text(encoding="utf-8"))
    rng = np.random.default_rng(seed)
    observations, offsets, actions, visits, results, movers, game_ids, terminated = [], [0], [], [], [], [], [], []
    completed = truncated = 0
    started = time.monotonic()
    for i in range(games):
        first, second = rng.integers(len(decks), size=2)
        game_seed = 2 * (seed + i) + i % 2
        game = RustHeadlessGame(decks[first]["cards"], decks[second]["cards"],
            seed=game_seed, observation_profile=PROFILE, record_actions=True)
        first_row = len(observations)
        decisions = 0
        try:
            while not game.is_game_over and decisions < 1200:
                mask = np.asarray(game.get_action_mask())
                legal = np.flatnonzero(mask > .5)
                if not len(legal):
                    raise RuntimeError("empty action mask")
                actor = game.current_player_id
                action = game.greedy_action()
                if action not in legal:
                    raise RuntimeError(f"greedy chose illegal action {action}")
                if len(legal) > 1:
                    observations.append(np.asarray(game.get_board_tensor(actor),dtype=np.float32).copy())
                    actions.extend(legal.tolist())
                    visits.extend((legal == action).astype(np.float32).tolist())
                    offsets.append(len(actions))
                    movers.append(actor)
                    game_ids.append(i)
                game.step(int(action))
                decisions += 1
        except BaseException:
            write_json(run_dir / "bootstrap_failure.json", {"game_index":i,"seed":game_seed,
                "decks":[int(first),int(second)],"recording":game.get_recording()})
            raise
        state = game.get_rl_state()
        ended = game.is_game_over and state.get("terminal_reason") not in ["step_limit", "crash"]
        completed += int(ended)
        truncated += int(not ended)
        winner = game.winner_id
        for mover in movers[first_row:]:
            results.append(0. if not ended or winner is None else (1. if mover == winner else -1.))
            terminated.append(int(ended))
        if (i+1) % 10 == 0:
            progress = {"games":i+1,"requested_games":games,"completed":completed,"truncated":truncated,
                "rows":len(observations),"elapsed_seconds":time.monotonic()-started}
            write_json(run_dir / "bootstrap_progress.json", progress)
            print(f"bootstrap {progress}",flush=True)
    directory = run_dir / "bootstrap_shards"
    shard = directory / "shard-000"
    shard.mkdir(parents=True, exist_ok=False)
    for name, array in dict(obs=np.asarray(observations,np.float32),policy_offsets=np.asarray(offsets,np.int64),
        policy_actions=np.asarray(actions,np.int32),policy_visits=np.asarray(visits,np.float32),
        z=np.asarray(results,np.float32),terminated=np.asarray(terminated,np.uint8),
        mover=np.asarray(movers,np.uint8),game_id=np.asarray(game_ids,np.int32)).items():
        np.save(shard / f"{name}.npy",array)
    contract=current_contract(PROFILE)
    write_json(directory / "manifest.json",dict(schema="selfplay-shards-v1",observation_profile=PROFILE,
        model_contract=contract,obs_dim=contract["tensor_size"],action_dim=contract["action_space_size"],
        value_target="terminal_outcome",policy_semantics="greedy demonstrations, not search targets",
        games=games,truncated_games=truncated,shards=[{"dir":"shard-000","rows":len(observations)}]))
    if truncated / games > .05:
        raise RuntimeError(f"Bootstrap truncation rate {truncated}/{games} exceeds 5%; inspect before training")
    return directory


def bootstrap(run_dir: Path, games: int, seed: int, device: str = "cuda", teacher_label: str = "greedy"):
    import gymnasium as gym
    import torch
    from sb3_contrib import MaskablePPO
    from digimon_gym.agents.features_extractor import CardEmbeddingExtractor
    from digimon_gym.agents.azero_trainer import train, calibrate_value
    from model_contract import current_contract
    from digimon_gym.tensor_profiles import get_tensor_profile
    from tools.export_onnx import export_mlp
    torch.set_num_threads(2)
    contract = current_contract(PROFILE)

    class Spaces(gym.Env):
        observation_space = gym.spaces.Box(-10.,20001.,(contract["tensor_size"],),np.float32)
        action_space = gym.spaces.Discrete(contract["action_space_size"])

    init = run_dir / "fresh_seed.zip"
    if not init.exists():
        model=MaskablePPO("MlpPolicy",Spaces(),n_steps=64,batch_size=64,seed=seed,device="cpu",
            policy_kwargs=dict(features_extractor_class=CardEmbeddingExtractor,
                features_extractor_kwargs=dict(features_dim=128,embedding_dim=8,tensor_profile_id=PROFILE),
                net_arch=dict(pi=[128,128],vf=[128,128])))
        model.digimon_contract=contract
        model.save(str(init))
    directory=run_dir / "bootstrap_shards"
    if not (directory / "manifest.json").exists():
        directory=collect_bootstrap(run_dir,games,seed)
    if teacher_label not in ("greedy", "development", "mixed"):
        raise ValueError("unknown bootstrap teacher label")
    imitation=run_dir / f"{teacher_label}_imitation.zip"
    if not imitation.exists():
        summary=train(init,[directory],imitation,epochs=4,batch_size=256,lr=2e-4,value_coef=0.,device=device,seed=seed)
        write_json(run_dir / "imitation_summary.json",summary)
    calibrated=run_dir / "seed.zip"
    if not calibrated.exists():
        calibrate_value(imitation,[directory],calibrated,epochs=8,batch_size=256,lr=5e-4,device=device,seed=seed)
    export_mlp(str(calibrated),str(run_dir / "seed.onnx"),get_tensor_profile(PROFILE))
    write_json(run_dir / "anchors.json",{"version":1,"champions":[{"name":"frozen-bootstrap",
        "weights_path":str(calibrated.resolve()),"algorithm":"mlp","observation_profile":PROFILE,
        "tensor_layout_hash":contract["tensor_layout_hash"],"source":f"fresh {teacher_label} imitation + terminal value fit"}]})


def check(run_dir: Path, cli_bin: Path, procs: int, seed: int):
    from tools.run_selfplay_generation import run_selfplay_stage
    reports = []
    for sims in [160, 320]:
        folder = run_dir / f"check-{sims}"
        if (folder / "summary.json").exists():
            reports.append(json.loads((folder / "summary.json").read_text(encoding="utf-8")))
            continue
        if folder.exists():
            raise FileExistsError(f"Preserving partial check at {folder}; investigate before retry")
        started=time.monotonic()
        directories=run_selfplay_stage(str(cli_bin),folder,str(run_dir / "seed.onnx"),False,
            run_dir / "decks.json",100,procs,sims,4,"pimc",seed,16,1200,4096,
            ROOT / "data/cards.json","implemented")
        manifests=[json.loads((p / "manifest.json").read_text(encoding="utf-8")) for p in directories]
        report={"simulations_total_per_decision":sims,"worlds":4,"games":sum(m["games"] for m in manifests),
            "evaluation_batch_size":manifests[0]["search"].get("evaluation_batch_size",1),
            "truncated_games":sum(m["truncated_games"] for m in manifests),
            "rows":sum(m["rows"] for m in manifests),"elapsed_seconds":time.monotonic()-started,
            "manifests":[str(p / "manifest.json") for p in directories]}
        report["games_per_hour"]=report["games"]*3600/report["elapsed_seconds"]
        report["positions_generated_per_second"]=report["rows"]/report["elapsed_seconds"]
        write_json(folder / "summary.json",report)
        reports.append(report)
    report={"games":sum(r["games"] for r in reports),"runs":reports,
        "purpose":"health and throughput; mirror self-play outcomes do not measure playing strength",
        "first_generation_simulations":320}
    if report["games"] != 200 or any(r["truncated_games"] / r["games"] > .05 for r in reports):
        write_json(run_dir / "check_failed.json",report)
        raise RuntimeError("200-game health gate failed; first generation must not start")
    write_json(run_dir / "check_summary.json",report)


def generation(run_dir: Path, cli_bin: Path, procs: int, seed: int):
    if not (run_dir / "check_summary.json").exists():
        raise ValueError("First finish the 200-game health check")
    command=[sys.executable,str(ROOT / "code/tools/run_selfplay_generation.py"),
        "--run-dir",str(run_dir / "generation"),"--seed-model",str(run_dir / "seed.zip"),
        "--seed-model-onnx",str(run_dir / "seed.onnx"),"--decks",str(run_dir / "decks.json"),
        "--cli-bin",str(cli_bin),"--cards-json",str(ROOT / "data/cards.json"),
        "--generations","1","--games","2000","--procs",str(procs),
        "--sims","320","--worlds","4","--decision-cap","1200", "--epochs","2",
        "--batch-size","512","--lr","0.0001","--train-device","cuda",
        "--eval-n","300","--deck-pool-snapshot",str(run_dir / "heldout_snapshot.json"),
        "--champions",str(run_dir / "anchors.json"),"--seed",str(seed+1000000)]
    subprocess.run(command,check=True)


def pipeline(run_dir: Path, cli_bin: Path, games: int, procs: int, seed: int):
    """Run the requested stages in order; preserve failures and never bypass the check."""
    if any((run_dir / name).exists() for name in
           ("provenance.json", "fresh_seed.zip", "check-160", "generation")):
        raise FileExistsError(f"Preserving existing run at {run_dir}; use a fresh directory")
    for phase, invoke in [
        ("prepare", lambda: prepare(run_dir, cli_bin)),
        ("bootstrap", lambda: bootstrap(run_dir, games, seed)),
        ("check", lambda: check(run_dir, cli_bin, procs, seed)),
        ("generation", lambda: generation(run_dir, cli_bin, procs, seed)),
    ]:
        write_json(run_dir / "pipeline_state.json", {"phase": phase, "status": "running"})
        print(f"=== English meta pipeline: {phase} ===", flush=True)
        try:
            invoke()
        except BaseException as exc:
            write_json(run_dir / "pipeline_state.json", {
                "phase": phase, "status": "failed", "error": str(exc)})
            raise
    write_json(run_dir / "pipeline_state.json", {
        "phase": "generation", "status": "completed", "promoted": False})


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("phase",choices=["prepare","bootstrap","check","generation","pipeline"])
    parser.add_argument("--run-dir",type=Path,required=True)
    parser.add_argument("--cli-bin",type=Path)
    parser.add_argument("--games",type=int,default=400)
    parser.add_argument("--seed",type=int,default=20261023)
    parser.add_argument("--procs",type=int,default=4)
    parser.add_argument("--bootstrap-device", choices=["cpu", "cuda"], default="cuda")
    args=parser.parse_args()
    os.environ["DIGIMON_CARDS_JSON"] = str(ROOT / "data/cards.json")
    os.environ["DIGIMON_EVAL_FAILURE_DIR"] = str((args.run_dir / "eval_failures").resolve())
    args.run_dir.mkdir(parents=True,exist_ok=True)
    if args.phase != "bootstrap" and args.cli_bin is None:
        parser.error(f"{args.phase} requires --cli-bin")
    if args.phase=="prepare":
        prepare(args.run_dir,args.cli_bin)
    elif args.phase=="bootstrap":bootstrap(args.run_dir,args.games,args.seed,args.bootstrap_device)
    elif args.phase=="check":check(args.run_dir,args.cli_bin,args.procs,args.seed)
    elif args.phase=="generation":generation(args.run_dir,args.cli_bin,args.procs,args.seed)
    else:pipeline(args.run_dir,args.cli_bin,args.games,args.procs,args.seed)
    return 0


if __name__=="__main__":
    raise SystemExit(main())
