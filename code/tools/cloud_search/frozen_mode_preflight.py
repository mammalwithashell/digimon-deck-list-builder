"""Excluded, single-attempt full-model qualification. This cannot launch scored panels."""
import argparse
from concurrent.futures import FIRST_COMPLETED, ThreadPoolExecutor, wait
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import time

try:
    from . import frozen_mode_receipts as receipts
except ImportError:
    import frozen_mode_receipts as receipts

CHECKPOINTS = (8, 12, 16, 20)
require = receipts.require


def read(path):
    return json.loads(Path(path).read_text(encoding="utf-8-sig"))


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write(path, value):
    path = Path(path)
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text(json.dumps(value, indent=2, allow_nan=False), encoding="utf-8")
    tmp.replace(path)


def schedule():
    """A cycle covers each list with both models in each physical seat.

    The same requests/seeds run on each OS. They are parity references, never
    independent strength evidence. Held-out games must actually observe fallback.
    """
    jobs = []
    for partition, count, modes in (("train", 21, ("closed-list", "known-list")),
                                    ("heldout", 23, ("closed-list",))):
        for mode in modes:
            for cp, opponent in ((8, 12), (16, 20)):
                for i in range(count):
                    jobs.append(dict(id=f"{partition}-{mode}-cp{cp}-cp{opponent}-list{i:02d}",
                        partition=partition, checkpoint=cp, opponent_checkpoint=opponent,
                        deck_indices=[i, (i+1) % count], candidate_mode=mode, opponent_mode=mode))
    for cp in CHECKPOINTS:
        jobs.append(dict(id=f"train-mixed-cp{cp}", partition="train", checkpoint=cp,
            opponent_checkpoint=cp, deck_indices=[0, 1], candidate_mode="closed-list", opponent_mode="known-list"))
    for i, job in enumerate(jobs):
        job.update(game_seed=11226103000+i, search_seed=11326103000+i)
    assert len(jobs) == 134
    return jobs


def settings(mode):
    return dict(kind="search", simulations=320, worlds=4, batch=4, root_noise=0, information_regime=mode)


def prepare(root, original, prior_root, runtime, provenance, source_manifest, platform, repo):
    require(not root.exists(), "existing preparation")
    proof = read(provenance)
    binary = "frozen_checkpoint_modes" + (".exe" if platform == "windows" else "")
    require(proof["platform"] == platform and proof["portable_policy_math"] is True, "platform/math")
    require(digest(runtime / binary) == proof["binary_sha256"], "binary provenance")
    require(proof["source_manifest_sha256"] == digest(source_manifest), "source provenance")
    # Freeze every declared source identity, including dependency lock and telemetry.
    source = read(source_manifest)["source_files"]
    require(all(digest(repo / p) == h for p, h in source.items()), "source changed since build")
    root.mkdir(parents=True)
    for folder in ("assets", "inputs", "requests", "plans", "results", "attempts", "logs"):
        (root / folder).mkdir()
    shutil.copytree(runtime, root / "runtime")
    shutil.copy2(__file__, root / "controller.py")
    shutil.copy2(Path(receipts.__file__), root / "frozen_mode_receipts.py")
    shutil.copy2(provenance, root / "inputs/build-provenance.json")
    shutil.copy2(source_manifest, root / "inputs/source-manifest.json")
    shutil.copy2(prior_root / "prior.json", root / "assets/prior.json")
    prior_proof = read(prior_root / "completion-verified.json")
    # The snapshot identity is supplied by production admission, not recomputed JSON hashing.
    require(digest(prior_root / "prior.json") == prior_proof["native_proof"]["prior_sha256"], "admitted prior changed")
    snapshot = prior_proof["native_proof"]["prior_snapshot_hash"]
    original_files = read(original / "campaign-plan.json")["files"]
    for path, expected in original_files.items():
        if path.startswith("assets/") and "checkpoint" in path:
            require(digest(original / path) == expected, "model/contract changed")
            shutil.copy2(original / path, root / path)
    for name in ("cards.json", "mechanics_readiness.json", "card_admissibility.json"):
        shutil.copy2(repo / "data" / name, root / "assets" / name)
    decks = {}
    for partition, expected in (("train", 21), ("heldout", 23)):
        path = prior_root / "sources" / (partition + "_snapshot.json")
        shutil.copy2(path, root / "inputs" / path.name)
        rows = read(path)["decks"]
        require(len(rows) == expected, "list coverage")
        decks[partition] = [dict(name=d["archetype"], cards=d["card_ids"]) for d in rows]
    jobs = schedule()
    write(root / "inputs/schedule.json", jobs)
    for job in jobs:
        cp, opp = job["checkpoint"], job["opponent_checkpoint"]
        plan_path = f"plans/{job['partition']}-cp{cp}-cp{opp}.json"
        plan = dict(schema=receipts.SCHEMA, native_sha256=proof["binary_sha256"],
            model=f"assets/checkpoint{cp}.onnx", opponent_model=f"assets/checkpoint{opp}.onnx",
            cards_json="assets/cards.json", prior="assets/prior.json", prior_snapshot_hash=snapshot,
            decks=decks[job["partition"]], decision_cap=2000)
        for key, field in (("model", "model_sha256"), ("opponent_model", "opponent_model_sha256"),
                           ("cards_json", "cards_sha256"), ("prior", "prior_file_sha256")):
            plan[field] = digest(root / plan[key])
        write(root / plan_path, plan)
        request = dict(schema=receipts.SCHEMA, id=job["id"], plan=plan_path,
            runtime_sha256=proof["binary_sha256"], prior_snapshot_hash=snapshot,
            deck_indices=job["deck_indices"], game_seed=job["game_seed"], search_seed=job["search_seed"],
            candidate=settings(job["candidate_mode"]), opponent=settings(job["opponent_mode"]))
        write(root / f"requests/{job['id']}.json", request)
    write(root / "protocol.json", dict(schema=receipts.SCHEMA, stage="excluded_full_model_preflight",
        scored=False, platform=platform, workers=3, games=268, requests=134, no_retry=True,
        deadline_seconds=7200, binary="runtime/"+binary, require_heldout_fallback=True,
        note="Two games per request; 3600 seconds per game allowance. Repeated OS references are correlated."))
    files = {p.relative_to(root).as_posix(): digest(p) for p in sorted(root.rglob("*")) if p.is_file()}
    write(root / "manifest.json", dict(files=files, created_at=time.time()))
    return {"root": str(root), "requests": 134, "excluded_games": 268}


def validate_inputs(root):
    manifest = read(root / "manifest.json")
    for path, expected in manifest["files"].items():
        resolved = (root / path).resolve()
        require(resolved.is_relative_to(root.resolve()) and digest(resolved) == expected, "frozen input: " + path)
    protocol = read(root / "protocol.json")
    require(protocol["stage"] == "excluded_full_model_preflight" and protocol["scored"] is False, "excluded only")
    require(protocol["workers"] == 3 and protocol["no_retry"] is True, "execution limits")
    require((protocol["games"], protocol["requests"], protocol["deadline_seconds"]) == (268,134,7200), "finite scope")
    jobs = read(root / "inputs/schedule.json")
    require(jobs == schedule(), "schedule coverage/seeds changed")
    for job in jobs:
        req = read(root / f"requests/{job['id']}.json")
        plan = read(root / req["plan"])
        require(req["id"] == job["id"], "request id")
        for key in ("game_seed", "search_seed", "deck_indices"):
            require(req[key] == job[key], "request schedule")
        for actor in ("candidate", "opponent"):
            require(req[actor] == settings(job[actor+"_mode"]), "requested mode/budget")
        require(plan["decision_cap"] == 2000 and req["runtime_sha256"] == plan["native_sha256"], "runtime/cap")
        require(plan["model"] == f"assets/checkpoint{job['checkpoint']}.onnx" and
                plan["opponent_model"] == f"assets/checkpoint{job['opponent_checkpoint']}.onnx", "model routing")
    return protocol, jobs


def execute(root, protocol, job):
    name = job["id"]
    request_path, output_path = f"requests/{name}.json", f"results/{name}.json"
    # Exclusive markers survive crashes and prohibit replay, including uncommitted attempts.
    with (root / f"attempts/{name}.json").open("x") as stream:
        json.dump(dict(at=time.time(), request_sha256=digest(root/request_path)), stream)
    try:
        with (root / f"logs/{name}.out.log").open("xb") as out, (root / f"logs/{name}.err.log").open("xb") as err:
            process = subprocess.run([str((root/protocol["binary"]).resolve()),request_path,output_path],
                cwd=root, stdout=out, stderr=err, timeout=protocol["deadline_seconds"], check=True)
        req = read(root/request_path)
        plan = read(root/req["plan"])
        counts = receipts.validate(req,read(root/output_path),plan)
        receipt = dict(schema=receipts.SCHEMA, request_sha256=digest(root/request_path),
            output_sha256=digest(root/output_path), runtime_sha256=plan["native_sha256"],
            prior_snapshot_hash=plan["prior_snapshot_hash"], counts=counts, committed_at=time.time())
        write(root/f"results/{name}.done.json",receipt)
        return True
    except Exception as error:
        write(root/f"results/{name}.rejected.json",dict(request=name,error=str(error),at=time.time(),
            request_sha256=digest(root/request_path), checkpoint=job["checkpoint"],
            opponent_checkpoint=job["opponent_checkpoint"], game_seed=job["game_seed"],
            search_seed=job["search_seed"],deck_indices=job["deck_indices"],native_output=output_path))
        return False


def run(root):
    protocol, jobs = validate_inputs(root)
    require(os.access(root/protocol["binary"], os.X_OK), "native executable permission missing before launch")
    # This controller has no resume path. A second invocation cannot launch work.
    with (root/"started.json").open("x") as stream:
        json.dump(dict(at=time.time(),pid=os.getpid()),stream)
    require(not any((root/"attempts").iterdir()) and not any((root/"results").iterdir()), "existing native evidence")
    failed, committed, cursor = False, 0, 0
    with ThreadPoolExecutor(max_workers=protocol["workers"]) as pool:
        active = set()
        while active or (cursor < len(jobs) and not failed):
            while not failed and len(active)<protocol["workers"] and cursor<len(jobs):
                active.add(pool.submit(execute,root,protocol,jobs[cursor])); cursor+=1
            done, active = wait(active,return_when=FIRST_COMPLETED)
            for future in done:
                try:
                    success = future.result()
                except Exception:
                    success = False
                failed |= not success
                committed += int(success)
            write(root/"state.json",dict(state="draining_failed" if failed else "running",
                committed_requests=committed, committed_games=2*committed, at=time.time(),pid=os.getpid()))
    if failed:
        write(root/"state.json",dict(state="failed",committed_requests=committed,at=time.time()))
        raise RuntimeError("preflight failed; no retries or scoring")
    verify(root)
    write(root/"state.json",dict(state="complete_verified",committed_games=268,at=time.time()))


def verify(root):
    protocol,jobs = validate_inputs(root)
    require(not list((root/"results").glob("*.rejected.json")), "rejected native evaluation")
    expected = {j["id"]+".done.json" for j in jobs}
    require({p.name for p in (root/"results").glob("*.done.json")} == expected, "incomplete receipts")
    hashes, fallback = {}, 0
    for job in jobs:
        name=job["id"]; req=read(root/f"requests/{name}.json"); plan=read(root/req["plan"])
        receipts.validate_receipt(root,f"requests/{name}.json",f"results/{name}.json",f"results/{name}.done.json",plan)
        hashes[name]=digest(root/f"results/{name}.done.json")
        if job["partition"]=="heldout":
            for game in read(root/f"results/{name}.json")["games"]:
                fallback+=sum(s["diagnostics"]["information"].get("fallback_worlds",0) for s in game["search_diagnostics"])
    require(fallback > 0, "heldout full games did not exercise population fallback; qualification incomplete")
    proof=dict(verified_at=time.time(),games=268,requests=134,scored=False,
        manifest_sha256=digest(root/"manifest.json"),receipt_hashes=hashes,heldout_fallback_worlds=fallback,
        cross_platform_parity_required=True,scored_launch_authorized=False)
    write(root/"completion-verified.json",proof)
    return {k:v for k,v in proof.items() if k!="receipt_hashes"}


def archive(root, destination):
    """Preserve empty queue directories and executable mode across Windows packaging."""
    protocol, _ = validate_inputs(root)
    require(not destination.exists(), "existing deployment archive")
    def mode(info):
        if info.name == protocol["binary"]:
            info.mode = 0o755
        return info
    with tarfile.open(destination, "w:gz") as bundle:
        for path in sorted(root.iterdir()):
            if path.resolve() != destination.resolve():
                bundle.add(path, arcname=path.name, filter=mode)
    with tarfile.open(destination) as bundle:
        require(bundle.getmember(protocol["binary"]).mode & 0o111, "archive executable mode")
        require(all(bundle.getmember(p).isdir() for p in ("results","attempts","logs")), "archive queue directories")
    return {"archive_sha256":digest(destination)}


if __name__=="__main__":
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command",choices=("prepare","run","verify","archive"))
    parser.add_argument("--root",type=Path,required=True)
    for key in ("original","prior-root","runtime","provenance","source-manifest","repo"):
        parser.add_argument("--"+key,type=Path)
    parser.add_argument("--platform",choices=("windows","linux"))
    parser.add_argument("--out",type=Path)
    args=parser.parse_args()
    if args.command=="prepare":
        result=prepare(args.root,args.original,args.prior_root,args.runtime,args.provenance,args.source_manifest,args.platform,args.repo)
    elif args.command=="run": result=run(args.root.resolve())
    elif args.command=="archive": result=archive(args.root,args.out)
    else: result=verify(args.root.resolve())
    print(json.dumps(result))
