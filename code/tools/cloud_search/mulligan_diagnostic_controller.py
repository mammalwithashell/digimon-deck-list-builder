"""Immutable, single-attempt known-list mulligan qualification sweeps.

This controller never trains, resumes failed requests, or launches scored games.
The full diagnostic remains gated on separately verified cross-platform parity.
"""
from __future__ import annotations

import argparse
from concurrent.futures import FIRST_COMPLETED, ThreadPoolExecutor, wait
import hashlib
import json
import math
import os
from pathlib import Path
import shutil
import subprocess
import time

CHECKPOINTS = (8, 12, 16, 20)
WORLDS = (4, 8, 16)
QUALIFICATION_SEED = 9826103099
CORPUS_HASH = "fcf23ce4c03f7fab9073a020e417147ecbfa7914617378a8a351e72ac94e585b"


def read(path):
    return json.loads(Path(path).read_text(encoding="utf-8-sig"))


def digest(path):
    h = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def write(path, value):
    path = Path(path)
    temp = path.with_suffix(path.suffix + ".tmp")
    temp.write_text(json.dumps(value, indent=2, allow_nan=False), encoding="utf-8")
    temp.replace(path)


def require(condition, message):
    if not condition:
        raise ValueError(message)


def prepare(root, campaign, corpus, runtime, provenance, platform):
    """Freeze all inputs; existing roots are intentionally rejected."""
    require(digest(corpus) == CORPUS_HASH, "cohort digest differs")
    nodes = read(corpus)["nodes"]
    require(len(nodes) == 208 and len({n["id"] for n in nodes}) == 208, "cohort size/identity")
    require({n["mover"] for n in nodes} == {0, 1}, "both mulligan seats required")
    original = read(campaign / "campaign-plan.json")
    asset_files = {p: h for p, h in original["files"].items() if p.startswith("assets/")}
    require(asset_files, "missing original asset manifest")
    for path, expected in asset_files.items():
        require(digest(campaign / path) == expected, f"original asset changed: {path}")
    binary_name = "mulligan_search_diagnostic_v2" + (".exe" if platform == "windows" else "")
    proof = read(provenance)
    require(digest(runtime / binary_name) == proof["binary_sha256"], "runtime provenance mismatch")
    require(proof["platform"] == platform, "runtime platform mismatch")
    root.mkdir(parents=True, exist_ok=False)
    for directory in ("inputs", "plans", "requests", "results", "logs", "attempts"):
        (root / directory).mkdir()
    for path in asset_files:
        dest = root / path
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(campaign / path, dest)
    shutil.copytree(runtime, root / "runtime")
    shutil.copy2(corpus, root / "inputs/corpus.json")
    shutil.copy2(provenance, root / "inputs/build-provenance.json")
    shutil.copy2(__file__, root / "controller.py")
    decks = read(campaign / "local/cp12-vs-cp8-primary/plan.json")["decks"]
    jobs = []
    for checkpoint in CHECKPOINTS:
        plan = dict(checkpoint=checkpoint, decks=decks, cards_json="assets/cards.json",
                    model=f"assets/checkpoint{checkpoint}.onnx", information_regime="known-list")
        write(root / f"plans/cp{checkpoint}.json", plan)
        for worlds in WORLDS:
            job = f"cp{checkpoint}-worlds{worlds}-qualification"
            request = dict(plan=f"plans/cp{checkpoint}.json", corpus="inputs/corpus.json",
                           worlds=worlds, simulations=320, seed=QUALIFICATION_SEED,
                           qualify=True, require_observation_hash=True)
            write(root / f"requests/{job}.json", request)
            jobs.append(job)
    protocol = dict(schema="mulligan-native-qualification-v1", platform=platform,
                    jobs=jobs, binary=f"runtime/{binary_name}", roots=208, decisions=2496,
                    workers=3, deadline_seconds=3600, scored=False,
                    information_regime="known-list", seed=QUALIFICATION_SEED,
                    purpose="All roots/models/world counts, exact reconstruction, observer and future-RNG invariance",
                    full_diagnostic_gate="Both platforms must pass and exact cross-platform parity must be verified",
                    no_retry=True, source_cohort_sha256=CORPUS_HASH)
    write(root / "protocol.json", protocol)
    files = {str(p.relative_to(root)).replace("\\", "/"): digest(p)
             for p in sorted(root.rglob("*")) if p.is_file()}
    write(root / "manifest.json", dict(created_at=time.time(), files=files))
    return dict(root=str(root), jobs=len(jobs), roots=208, decisions=2496)


def validate_inputs(root):
    manifest = read(root / "manifest.json")
    for path, expected in manifest["files"].items():
        require(digest(root / path) == expected, f"frozen input changed: {path}")
    protocol = read(root / "protocol.json")
    if protocol.get("stage") == "full_diagnostic":
        parity = read(root / "inputs/parity-verified.json")
        require(parity["passed"] is True and not parity["differences"], "cross-platform parity gate failed")
        require(parity["decisions_compared"] == 2496, "qualification coverage incomplete")
        require(protocol["qualification_manifests"] == {
            "windows": parity["windows_manifest"], "linux": parity["linux_manifest"]}, "qualification provenance mismatch")
        qualified_manifest = read(root / "inputs/qualified-manifest.json")
        require(digest(root / "inputs/qualified-manifest.json") == protocol["qualification_manifests"][protocol["platform"]], "qualified runtime manifest changed")
        for path, expected in qualified_manifest["files"].items():
            if path.startswith(("assets/", "runtime/")) or path in ("inputs/corpus.json", "inputs/build-provenance.json"):
                require(manifest["files"].get(path) == expected, f"full diagnostic differs from qualified input: {path}")
    return protocol


def prepare_full(root, windows, linux, parity_path, platform):
    """Freeze a host shard only after BOTH qualified results and exact parity."""
    require(not root.exists(), "existing full diagnostic root")
    parity = read(parity_path)
    require(parity["passed"] is True and not parity["differences"] and parity["decisions_compared"] == 2496, "parity gate")
    for host, qualification in (("windows", windows), ("linux", linux)):
        verify(qualification)
        require(digest(qualification / "manifest.json") == parity[host + "_manifest"], "qualification manifest changed")
        for job, hashes in parity["sweep_outputs"].items():
            require(digest(qualification / f"results/{job}.json") == hashes[host], "qualified output changed")
    qualification = windows if platform == "windows" else linux
    source_protocol = read(qualification / "protocol.json")
    require(source_protocol["platform"] == platform, "platform mismatch")
    root.mkdir(parents=True, exist_ok=False)
    for name in ("assets", "runtime", "plans", "inputs"):
        shutil.copytree(qualification / name, root / name)
    for name in ("requests", "results", "logs", "attempts"):
        (root / name).mkdir()
    shutil.copy2(__file__, root / "controller.py")
    shutil.copy2(parity_path, root / "inputs/parity-verified.json")
    shutil.copy2(qualification / "manifest.json", root / "inputs/qualified-manifest.json")
    seeds = [9826103100] if platform == "windows" else [9826103101, 9826103102]
    jobs = []
    for checkpoint in CHECKPOINTS:
        for worlds in WORLDS:
            for seed in seeds:
                job = f"cp{checkpoint}-worlds{worlds}-seed{seed}"
                write(root / f"requests/{job}.json", dict(plan=f"plans/cp{checkpoint}.json", corpus="inputs/corpus.json",
                      worlds=worlds, simulations=320, seed=seed, qualify=False, require_observation_hash=True))
                jobs.append(job)
    protocol = dict(source_protocol, schema="mulligan-native-full-v1", stage="full_diagnostic", jobs=jobs,
                    seeds=seeds, seed=None, decisions=208*len(jobs),
                    purpose="Frozen matched roots; policy and search preferences, not strength or expert correctness",
                    qualification_manifests={h: parity[h + "_manifest"] for h in ("windows", "linux")},
                    cross_host_timing="Report separately; Windows seed 3100, Linux seeds 3101/3102. Seeds paired within models/settings.")
    write(root / "protocol.json", protocol)
    write(root / "manifest.json", dict(created_at=time.time(), files={str(p.relative_to(root)).replace("\\", "/"): digest(p)
          for p in sorted(root.rglob("*")) if p.is_file()}))
    validate_inputs(root)
    return dict(root=str(root), platform=platform, sweeps=len(jobs), decisions=protocol["decisions"])


def validate_output(request, output, nodes):
    require(output["request"] == request, "request echo mismatch")
    rows = output["rows"]
    require(len(rows) == len(nodes), "incomplete result")
    for node, row in zip(nodes, rows):
        require(row["node"] == node["id"], "root identity/order mismatch")
        require(row["archetype"] == node["archetype"], "archetype mismatch")
        require(row["observation_sha256"] == node["observation_sha256"], "observation mismatch")
        require(row["simulations_run"] == request["simulations"], "simulation budget mismatch")
        require(row["observer_and_future_rng_invariance_verified"] is request["qualify"], "qualification missing")
        require(row["selected_plan"] in ([0], [1]), "invalid mulligan plan")
        plans = row["root_plans"]
        require({tuple(p[0]) for p in plans} == {(0,), (1,)} and len(plans) == 2, "root plans incomplete")
        require(all(isinstance(p[1], int) and p[1] >= 0 and math.isfinite(p[2]) for p in plans), "invalid root plan values")
        require(row["selected_plan"] == max(enumerate(plans), key=lambda item: (item[1][1], -item[0]))[1][0], "selected plan inconsistent")
        require(dict(row["root_visits"]) == {p[0][0]: p[1] for p in plans}, "visit projection mismatch")
        require(set(dict(row["root_q"])) == {0, 1}, "missing root Q")
        for action, value in row["root_q"]:
            expected = next(p[2] if p[1] else 0 for p in plans if p[0] == [action])
            require(math.isfinite(value) and abs(value - expected) < 1e-6, "root Q mismatch")
        samples = row["sampled_worlds"]
        require(len(samples) == request["worlds"], "wrong sampled-world count")
        require(len({s["seed"] for s in samples}) == len(samples), "duplicate world seeds")
        for sample in samples:
            require(len(sample["replacement_hand"]) == len(node["hand"]), "replacement hand size")
        unique = len({tuple(sorted(s["replacement_hand"])) for s in samples})
        require(row["unique_replacement_multisets"] == unique, "replacement diversity mismatch")
        for key in ("keep_prior", "redraw_prior", "policy_value", "root_value", "seconds"):
            require(math.isfinite(row[key]), f"nonfinite {key}")
        require(row["seconds"] >= 0, "negative duration")
        require(0 <= row["keep_prior"] <= 1 and 0 <= row["redraw_prior"] <= 1, "invalid priors")
        require(abs(row["keep_prior"] + row["redraw_prior"] - 1) < 1e-5, "unnormalized priors")
        values = row["evaluated_nodes_including_roots"]
        require(values["count"] > 0, "missing evaluated-node values")
        require(all(math.isfinite(values[k]) for k in ("mean", "min", "max")), "nonfinite node values")
        require(values["min"] <= values["mean"] + 1e-7 <= values["max"] + 2e-7, "invalid value range")
    return len(rows)


def execute(root, protocol, job, nodes):
    request_path = root / f"requests/{job}.json"
    output_path = root / f"results/{job}.json"
    attempt_path = root / f"attempts/{job}.json"
    # Exclusive attempt marker prevents any retry after native invocation.
    with attempt_path.open("x", encoding="utf-8") as stream:
        json.dump(dict(started_at=time.time(), pid=os.getpid(), request_sha256=digest(request_path)), stream)
    started = time.time()
    try:
        require(not output_path.exists(), "existing output")
        with (root / f"logs/{job}.out.log").open("wb") as out, (root / f"logs/{job}.err.log").open("wb") as err:
            result = subprocess.run([str(root / protocol["binary"]), f"requests/{job}.json", f"results/{job}.json"],
                                    cwd=root, stdout=out, stderr=err, timeout=protocol["deadline_seconds"])
        require(result.returncode == 0, f"native exit {result.returncode}")
        count = validate_output(read(request_path), read(output_path), nodes)
        receipt = dict(job=job, roots=count, started_at=started, completed_at=time.time(),
                       request_sha256=digest(request_path), output_sha256=digest(output_path),
                       manifest_sha256=digest(root / "manifest.json"))
        write(root / f"results/{job}.done.json", receipt)
        return True
    except Exception as error:
        write(root / f"results/{job}.rejected.json", dict(job=job, started_at=started, failed_at=time.time(),
              error=str(error), request=f"requests/{job}.json", request_sha256=digest(request_path),
              output_sha256=digest(output_path) if output_path.exists() else None,
              stderr=f"logs/{job}.err.log", retry_allowed=False))
        return False


def run(root):
    root = root.resolve()
    protocol = validate_inputs(root)
    require(not list((root / "attempts").glob("*")), "prior attempts exist; no automatic resume")
    with (root / "started.json").open("x", encoding="utf-8") as stream:
        json.dump(dict(time=time.time(), pid=os.getpid(), manifest_sha256=digest(root / "manifest.json")), stream)
    nodes = read(root / "inputs/corpus.json")["nodes"]
    jobs = iter(protocol["jobs"])
    failed, committed = False, 0
    write(root / "state.json", dict(state="running", committed_sweeps=0, time=time.time(), pid=os.getpid()))
    with ThreadPoolExecutor(max_workers=protocol["workers"]) as pool:
        pending = {pool.submit(execute, root, protocol, next(jobs), nodes) for _ in range(protocol["workers"])}
        while pending:
            finished, pending = wait(pending, return_when=FIRST_COMPLETED)
            for future in finished:
                try:
                    ok = future.result()
                except Exception:
                    ok = False
                failed |= not ok
                committed += int(ok)
            write(root / "state.json", dict(state="draining_after_failure" if failed else "running",
                  committed_sweeps=committed, committed_decisions=committed * len(nodes), time=time.time(), pid=os.getpid()))
            if not failed:
                for _ in finished:
                    job = next(jobs, None)
                    if job is not None:
                        pending.add(pool.submit(execute, root, protocol, job, nodes))
    write(root / "state.json", dict(state="failed" if failed else "complete_unverified", time=time.time(),
          committed_sweeps=committed, committed_decisions=committed * len(nodes), pid=os.getpid()))
    if not failed:
        verify(root)
    return not failed


def verify(root):
    protocol = validate_inputs(root)
    require(not list((root / "results").glob("*.rejected.json")), "rejected sweep")
    receipts = list((root / "results").glob("*.done.json"))
    require(len(receipts) == len(protocol["jobs"]), "incomplete receipts")
    nodes = read(root / "inputs/corpus.json")["nodes"]
    hashes = {}
    for job in protocol["jobs"]:
        receipt = read(root / f"results/{job}.done.json")
        require(receipt["manifest_sha256"] == digest(root / "manifest.json"), "manifest receipt mismatch")
        for kind, folder in (("request", "requests"), ("output", "results")):
            require(receipt[kind + "_sha256"] == digest(root / f"{folder}/{job}.json"), f"{kind} receipt mismatch")
        require(receipt["job"] == job and receipt["roots"] == len(nodes), "receipt identity/count mismatch")
        validate_output(read(root / f"requests/{job}.json"), read(root / f"results/{job}.json"), nodes)
        hashes[job] = digest(root / f"results/{job}.done.json")
    proof = dict(verified_at=time.time(), manifest_sha256=digest(root / "manifest.json"),
                 platform=protocol["platform"], sweeps=len(hashes), roots=len(nodes),
                 decisions=len(hashes)*len(nodes), receipt_sha256=hashes,
                 cross_platform_parity="verified_before_launch" if protocol.get("stage") == "full_diagnostic" else "not_yet_verified",
                 full_diagnostic_authorized_by_this_proof=False)
    write(root / "completion-verified.json", proof)
    return proof


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=("prepare", "prepare-full", "run", "verify"))
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--campaign", type=Path)
    parser.add_argument("--corpus", type=Path)
    parser.add_argument("--runtime", type=Path)
    parser.add_argument("--provenance", type=Path)
    parser.add_argument("--platform", choices=("windows", "linux"))
    parser.add_argument("--qualification-windows", type=Path)
    parser.add_argument("--qualification-linux", type=Path)
    parser.add_argument("--parity-proof", type=Path)
    args = parser.parse_args()
    if args.command == "prepare":
        result = prepare(args.root, args.campaign, args.corpus, args.runtime, args.provenance, args.platform)
    elif args.command == "prepare-full":
        result = prepare_full(args.root, args.qualification_windows, args.qualification_linux, args.parity_proof, args.platform)
    elif args.command == "run":
        result = {"success": run(args.root)}
    else:
        proof = verify(args.root)
        result = {k: proof[k] for k in ("platform", "sweeps", "roots", "decisions")}
    print(json.dumps(result))
    if result.get("success") is False:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
