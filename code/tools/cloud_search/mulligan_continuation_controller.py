"""Single-attempt qualification and parity-gated paired continuations.

All preparation is in new roots. Rejected/attempted native jobs are never retried.
Validation checks recorded semantics; native reconstruction/legality tests and
cross-platform game parity remain separate qualification requirements.
"""
from concurrent.futures import FIRST_COMPLETED, ThreadPoolExecutor, wait
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import shutil
import subprocess
import time

SCHEMA = "mulligan-continuation-game-v1"
CHECKPOINTS = (8, 12, 16, 20)
TERMINALS = {"Some(SecurityAttack)", "Some(DeckOut)", "Some(EngineDeclared)",
             "Some(Concede)", "Some(InfiniteLoopDraw)"}


def read(path):
    return json.loads(Path(path).read_text(encoding="utf-8-sig"))


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def write(path, value):
    path = Path(path)
    temp = path.with_suffix(path.suffix + ".tmp")
    temp.write_text(json.dumps(value, indent=2, allow_nan=False), encoding="utf-8")
    temp.replace(path)


def finite(value):
    return type(value) in (int, float) and math.isfinite(value) and value >= 0


def validate_output(request, output, plan, node):
    require(all(v.get("schema") == SCHEMA for v in (request, plan, output)), "schema")
    require(output["request"] == request and output["plan_sha256"] == request["plan_sha256"], "request/plan echo")
    require(request["runtime_sha256"] == output["runtime_sha256"] == plan["runtime_sha256"], "runtime")
    require(request["protocol_sha256"] == plan["protocol_sha256"], "protocol")
    require(request["checkpoint"] == plan["checkpoint"] in CHECKPOINTS, "checkpoint")
    require(request["opponent_checkpoint"] == plan["opponent_checkpoint"] == 8, "fixed opponent")
    require(plan["information_regime"] == "known-list", "information regime")
    require([plan[k] for k in ("simulations", "worlds", "batch", "root_noise", "decision_cap", "deadline_seconds")] == [320, 4, 4, 0, 2000, 3600], "settings")
    for key in ("model_sha256", "opponent_model_sha256"):
        require(output[key] == plan[key], key)
    require(request["root"] == node["id"] and output["root_observation_sha256"] == node["observation_sha256"], "root")
    game = output["game"]
    require(type(node["first"]) is bool, "root opener convention")
    first_player = node["mover"] if node["first"] else 1 - node["mover"]
    require(game["candidate_seat"] == node["mover"] in (0, 1) and game["first_player"] == first_player, "seat/opener")
    require(game["truncated"] is False and game["terminal_reason"] in TERMINALS, "terminal")
    winner = game["winner"]
    require(winner is None or (type(winner) is int and winner in (0, 1)), "winner")
    require((winner is None) == (game["terminal_reason"] == "Some(InfiniteLoopDraw)"), "draw semantics")
    require(game["score"] == (0.5 if winner is None else float(winner == node["mover"])), "score")
    require(finite(game["seconds"]) and game["seconds"] <= 3600, "deadline")
    actions, steps = game["actions"], game["card_metrics"]["steps"]
    require(game["card_metrics"]["initial_ui"][f"player{node['mover']+1}"]["handIds"] == node["hand"], "initial hand")
    require(1 < len(actions) == game["decisions"] == len(steps) <= 2000, "trace length")
    branch = request["forced_mulligan_action"]
    require(type(branch) is int and branch in (0, 1) and actions[0] == branch, "forced branch")
    require(steps[0]["phase"] == "Mulligan" and steps[0]["mover"] == node["mover"] + 1
            and steps[0]["legal_actions"] == [0, 1] and steps[0].get("forced_intervention") is True, "forced root")
    for index, (action, step) in enumerate(zip(actions, steps)):
        require(type(action) is int and action == step["action"] and action in step["legal_actions"], "recorded legality")
        require(step["mover"] in (1, 2) and finite(step["selection_seconds"]), "actor/timing")
        require(index == 0 or not step.get("forced_intervention", False), "extra forced decision")
    covered = set()
    searches = game["search_diagnostics"]
    require(game["searched"] == len(searches), "search count")
    for search in searches:
        start, route, actor = search["decision"], search["selected_plan"], search["actor"]
        require(type(start) is int and 1 <= start < len(actions) and route, "search position")
        require(len(steps[start]["legal_actions"]) > 1, "searched forced root")
        span = set(range(start, start + len(route)))
        require(start + len(route) <= len(actions) and not covered.intersection(span), "search overlap")
        require(actions[start:start + len(route)] == route, "executed plan")
        covered.update(span)
        require(actor in (0, 1) and all(s["mover"] == actor + 1 for s in steps[start:start + len(route)]), "route actor")
        expected_model = plan["model_sha256" if actor == node["mover"] else "opponent_model_sha256"]
        require(search["model_sha256"] == expected_model, "model routing")
        require([search[k] for k in ("simulations", "worlds", "batch", "root_noise")] == [320, 4, 4, 0], "actual budget")
        require(search["information_regime"] == "known-list", "actual regime")
        seed = (request["follow_on_search_seed"] + start * 0x9e3779b97f4a7c15 + actor) % (1 << 64)
        require(search["search_seed"] == seed, "independent search seed")
        plans = search["root_plans"]
        require(plans and route == max(enumerate(plans), key=lambda p: (p[1][1], -p[0]))[1][0], "deterministic selection")
        opportunity = search["opportunity"]
        require(opportunity["selected_plan"] == route, "opportunity selection")
        require(opportunity.get("executed_actions") == route and opportunity.get("commitment_executed") is True,
                "opportunity execution")
        require(opportunity["schema"] == "search-opportunities-v1" and opportunity["actor"] == actor
                and opportunity["sampling"] == "argmax_tau0" and opportunity["simulations_run"] == 320, "opportunity context")
        require(len(opportunity["plans"]) == len(plans), "opportunity coverage")
        for row, native in zip(opportunity["plans"], plans):
            require(row["plan"] == native[0] and row["visits"] == native[1], "opportunity visits")
            require(finite(row["prior_before_noise"]) and finite(row["prior_after_noise"]), "missing/nonfinite prior")
            require(row["prior_before_noise"] == row["prior_after_noise"], "unexpected root noise")
            require(row["selected"] == (row["plan"] == route) and row["selection_probability"] == float(row["selected"]), "opportunity choice")
            require(row["q"] == (native[2] if native[1] else None), "opportunity Q")
    require(all(i == 0 or len(s["legal_actions"]) == 1 or i in covered for i, s in enumerate(steps)), "unsearched nonforced decision")
    return {"games": 1, "score": game["score"], "decisions": len(actions)}


def prepare(root, original, preregistration, corpus, runtime, provenance, platform, repo):
    root, original, preregistration, corpus, runtime, provenance, repo = map(Path,
        (root, original, preregistration, corpus, runtime, provenance, repo))
    require(not root.exists(), "existing preparation")
    registration = read(preregistration / "manifest.json")
    for name, expected in registration["files"].items():
        require(digest(preregistration / name) == expected, "preregistration changed")
    protocol = read(preregistration / "protocol.json")
    require(protocol["scored_games"] == 832 and protocol["excluded_qualification_game_cap"] == 16, "finite budget")
    require(digest(corpus) == protocol["source_corpus_sha256"], "source roots")
    selected = {n["id"] for n in read(preregistration / "selected-roots.json")}
    remaining = [n for n in read(corpus)["nodes"] if n["id"] not in selected]
    node = min(remaining, key=lambda n: hashlib.sha256(("continuation-qualification-v1/" + n["id"]).encode()).hexdigest())
    proof = read(provenance)
    binary = "mulligan_continuation" + (".exe" if platform == "windows" else "")
    require(proof["platform"] == platform and proof["portable_policy_math"] is True, "build platform/math")
    require(digest(runtime / binary) == proof["binary_sha256"], "binary proof")
    root.mkdir(parents=True)
    for folder in ("inputs", "plans", "requests", "results", "attempts", "logs", "assets"):
        (root / folder).mkdir()
    shutil.copytree(runtime, root / "runtime")
    shutil.copy2(provenance, root / "inputs/build-provenance.json")
    shutil.copy2(__file__, root / "controller.py")
    shutil.copy2(preregistration / "protocol.json", root / "inputs/preregistered-protocol.json")
    shutil.copy2(preregistration / "selected-roots.json", root / "inputs/scored-roots.json")
    write(root / "inputs/root.json", node)
    # Old model files are inputs, not revalidation of completed old games.
    assets = read(original / "campaign-plan.json")["files"]
    for name, expected in assets.items():
        if name.startswith("assets/") and ("checkpoint" in name or name.endswith("cards.json")):
            require(digest(original / name) == expected, "original model asset changed")
            shutil.copy2(original / name, root / name)
    # New runtime uses current cards/admission evidence, explicitly frozen here.
    for name in ("cards.json", "mechanics_readiness.json", "card_admissibility.json"):
        shutil.copy2(repo / "data" / name, root / "assets" / name)
    decks = read(original / "local/cp12-vs-cp8-primary/plan.json")["decks"]
    jobs = []
    for cp in CHECKPOINTS:
        model = f"assets/checkpoint{cp}.onnx"
        require(digest(root / model) == protocol["models"][str(cp)], "preregistered model")
        plan = dict(schema=SCHEMA, checkpoint=cp, opponent_checkpoint=8, decks=decks,
            model=model, model_sha256=digest(root / model), opponent_model="assets/checkpoint8.onnx",
            opponent_model_sha256=protocol["models"]["8"], cards_json="assets/cards.json",
            cards_sha256=digest(root / "assets/cards.json"), runtime_sha256=proof["binary_sha256"],
            protocol="inputs/preregistered-protocol.json", protocol_sha256=digest(root / "inputs/preregistered-protocol.json"),
            information_regime="known-list", simulations=320, worlds=4, batch=4, root_noise=0,
            decision_cap=2000, deadline_seconds=3600)
        write(root / f"plans/cp{cp}.json", plan)
        for branch in (0, 1):
            job = f"qualification-cp{cp}-action{branch}"
            request = dict(schema=SCHEMA, id=job, stage="excluded_qualification", checkpoint=cp,
                opponent_checkpoint=8, root=node["id"], root_path="inputs/root.json",
                root_sha256=digest(root / "inputs/root.json"), forced_mulligan_action=branch,
                outcome_seed=10226103000, follow_on_search_seed=10326103000,
                plan=f"plans/cp{cp}.json", plan_sha256=digest(root / f"plans/cp{cp}.json"),
                protocol_sha256=plan["protocol_sha256"], runtime_sha256=proof["binary_sha256"])
            write(root / f"requests/{job}.json", request)
            jobs.append(job)
    write(root / "protocol.json", dict(schema=SCHEMA, stage="excluded_qualification", platform=platform,
        jobs=jobs, workers=3, games=8, scored=False, deadline_seconds=3600,
        binary="runtime/" + binary, root=node["id"], no_retry=True))
    files = {p.relative_to(root).as_posix(): digest(p) for p in sorted(root.rglob("*")) if p.is_file()}
    write(root / "manifest.json", dict(files=files, created_at=time.time()))
    return {"root": str(root), "jobs": 8, "scored": False, "selected_root": node["id"]}


def validate_inputs(root):
    for path, expected in read(root / "manifest.json")["files"].items():
        require(digest(root / path) == expected, "frozen input changed: " + path)
    protocol = read(root / "protocol.json")
    require(protocol["stage"] in ("excluded_qualification", "scored_continuations"), "unknown stage")
    scored = protocol["stage"] == "scored_continuations"
    count = 416 if scored else 8
    require(protocol["scored"] is scored and protocol["games"] == len(protocol["jobs"]) == len(set(protocol["jobs"])) == count, "game coverage")
    require(protocol["workers"] <= 3 and protocol["no_retry"] is True, "execution limits")
    selected = {n["id"]: n for n in read(root / "inputs/scored-roots.json")}
    if scored:
        parity = read(root / "inputs/qualification-parity.json")
        require(parity["passed"] is True and parity["differences"] == [] and parity["games_compared"] == 8, "full qualification parity gate")
        for host in ("windows", "linux"):
            require(digest(root / f"inputs/{host}-qualification-manifest.json") == parity["qualification_manifests"][host], "qualification manifest binding")
        qualified = read(root / f"inputs/{protocol['platform']}-qualification-manifest.json")["files"]
        current = read(root / "manifest.json")["files"]
        for path, expected in qualified.items():
            if path.startswith(("assets/", "runtime/")) or path == "inputs/build-provenance.json":
                require(current.get(path) == expected, "full differs from qualified runtime/input: " + path)
        expected_schedule = read(root / "inputs/scored-schedule.json")
        require(len(selected) == 52 and protocol["repeat"] in (0, 1), "full roots/repeat")
        require(protocol["repeat"] == (0 if protocol["platform"] == "windows" else 1), "paired host allocation")
        expected_jobs = {j["id"]: j for j in expected_schedule if j["repeat"] == protocol["repeat"]}
        require(set(expected_jobs) == set(protocol["jobs"]), "frozen shard schedule")
    else:
        node = read(root / "inputs/root.json")
        require(node["id"] not in selected, "qualification overlaps scored roots")
    combinations = set()
    for job in protocol["jobs"]:
        request = read(root / f"requests/{job}.json")
        require(request["stage"] == protocol["stage"], "request stage")
        if scored:
            require(all(request[k] == v for k, v in expected_jobs[job].items()), "preregistered request changed")
            require(read(root / request["root_path"]) == selected[request["root"]], "preregistered root changed")
            combinations.add((request["checkpoint"], request["root"], request["forced_mulligan_action"]))
        else:
            require(request["root"] == node["id"], "request qualification identity")
            combinations.add((request["checkpoint"], request["forced_mulligan_action"]))
        require(digest(root / request["plan"]) == request["plan_sha256"], "plan digest")
        require(digest(root / request["root_path"]) == request["root_sha256"], "root digest")
    expected = {(cp, n, b) for cp in CHECKPOINTS for n in selected for b in (0, 1)} if scored else {(cp, b) for cp in CHECKPOINTS for b in (0, 1)}
    require(combinations == expected, "model/root/branch coverage")
    return protocol


def prepare_full(root, windows, linux, parity_path, preregistration, platform):
    """Each host takes one outcome repeat for ALL roots/models; both branches stay together."""
    require(not root.exists(), "existing full root")
    parity = read(parity_path)
    require(parity["passed"] is True and parity["differences"] == [] and parity["games_compared"] == 8, "exact parity gate")
    for host, qualification in (("windows", windows), ("linux", linux)):
        # These are input gates, not rerunning any completed native evaluation.
        require(digest(qualification / "manifest.json") == parity["qualification_manifests"][host], "qualification identity")
        proof = read(qualification / "completion-verified.json")
        require(proof["games"] == 8 and proof["manifest_sha256"] == parity["qualification_manifests"][host], "qualification incomplete")
        require(not list((qualification / "results").glob("*.rejected.json")), "qualification rejection")
        require(set(proof["receipts"]) == set(parity["outputs"]), "qualification receipt coverage")
        for job, hashes in parity["outputs"].items():
            require(digest(qualification / f"results/{job}.json") == hashes[host], "qualified output changed")
            require(digest(qualification / f"results/{job}.done.json") == proof["receipts"][job], "qualification receipt changed")
    require(len(parity["outputs"]) == 8, "parity receipt coverage")
    require(platform in ("windows", "linux"), "platform")
    source = windows if platform == "windows" else linux
    old = validate_inputs(source)
    registration = read(preregistration / "manifest.json")
    for name, expected in registration["files"].items():
        require(digest(preregistration / name) == expected, "preregistration changed")
    require(digest(preregistration / "protocol.json") == digest(source / "inputs/preregistered-protocol.json"), "changed scored protocol")
    require(digest(preregistration / "selected-roots.json") == digest(source / "inputs/scored-roots.json"), "changed scored roots")
    nodes = read(preregistration / "selected-roots.json")
    schedule = read(preregistration / "jobs.json")
    require(len(nodes) == 52 and len(schedule) == 832, "finite scored scope")
    repeat = 0 if platform == "windows" else 1
    jobs = [j for j in schedule if j["repeat"] == repeat]
    require(len(jobs) == 416, "shard size")
    root.mkdir(parents=True)
    for folder in ("inputs", "plans", "requests", "results", "attempts", "logs"):
        (root / folder).mkdir()
    for folder in ("assets", "runtime"):
        shutil.copytree(source / folder, root / folder)
    shutil.copy2(source / "inputs/build-provenance.json", root / "inputs/build-provenance.json")
    shutil.copy2(preregistration / "protocol.json", root / "inputs/preregistered-protocol.json")
    shutil.copy2(preregistration / "selected-roots.json", root / "inputs/scored-roots.json")
    shutil.copy2(preregistration / "jobs.json", root / "inputs/scored-schedule.json")
    shutil.copy2(parity_path, root / "inputs/qualification-parity.json")
    for host, qualification in (("windows", windows), ("linux", linux)):
        shutil.copy2(qualification / "manifest.json", root / f"inputs/{host}-qualification-manifest.json")
    shutil.copy2(__file__, root / "controller.py")
    for cp in CHECKPOINTS:
        shutil.copy2(source / f"plans/cp{cp}.json", root / f"plans/cp{cp}.json")
    for node in nodes:
        write(root / f"inputs/{node['id']}.json", node)
    for job in jobs:
        cp = job["checkpoint"]
        plan = read(root / f"plans/cp{cp}.json")
        request = dict(job, schema=SCHEMA, stage="scored_continuations", plan=f"plans/cp{cp}.json",
            plan_sha256=digest(root / f"plans/cp{cp}.json"), runtime_sha256=plan["runtime_sha256"],
            protocol_sha256=plan["protocol_sha256"], root_path=f"inputs/{job['root']}.json",
            root_sha256=digest(root / f"inputs/{job['root']}.json"))
        write(root / f"requests/{job['id']}.json", request)
    write(root / "protocol.json", dict(schema=SCHEMA, stage="scored_continuations", scored=True, platform=platform,
        jobs=[j["id"] for j in jobs], games=416, workers=3, repeat=repeat, deadline_seconds=3600,
        binary=old["binary"], no_retry=True, allocation="Windows repeat0/Linux repeat1; both branches per pair stay on one host"))
    files = {p.relative_to(root).as_posix(): digest(p) for p in sorted(root.rglob("*")) if p.is_file()}
    write(root / "manifest.json", dict(files=files, created_at=time.time()))
    validate_inputs(root)
    return dict(root=str(root), games=416, scored=True, repeat=repeat)


def execute(root, protocol, job):
    request_path, output_path = root / f"requests/{job}.json", root / f"results/{job}.json"
    with (root / f"attempts/{job}.json").open("x", encoding="utf-8") as stream:
        json.dump({"started_at": time.time(), "request_sha256": digest(request_path)}, stream)
    try:
        with (root / f"logs/{job}.out.log").open("x") as stdout, (root / f"logs/{job}.err.log").open("x") as stderr:
            proc = subprocess.run([str(root / protocol["binary"]), str(request_path), str(output_path)],
                cwd=root, stdout=stdout, stderr=stderr, timeout=protocol["deadline_seconds"], check=False)
        require(proc.returncode == 0, f"native exit {proc.returncode}")
        request = read(request_path)
        stats = validate_output(request, read(output_path), read(root / request["plan"]), read(root / request["root_path"]))
        write(root / f"results/{job}.done.json", dict(job=job, request_sha256=digest(request_path),
            output_sha256=digest(output_path), manifest_sha256=digest(root / "manifest.json"), **stats))
        return True
    except Exception as error:
        write(root / f"results/{job}.rejected.json", dict(job=job, error=str(error),
            request_sha256=digest(request_path), output_sha256=digest(output_path) if output_path.exists() else None,
            retry_allowed=False, failed_at=time.time(), evidence=f"results/{job}.active.json"))
        return False


def run(root):
    root = root.resolve()
    protocol = validate_inputs(root)
    require(not list((root / "attempts").iterdir()), "prior attempt; no resume")
    with (root / "started.json").open("x", encoding="utf-8") as stream:
        json.dump(dict(pid=os.getpid(), at=time.time(), manifest_sha256=digest(root / "manifest.json")), stream)
    write(root / "state.json", dict(state="running", committed_games=0, at=time.time(), pid=os.getpid()))
    jobs, failed, committed = iter(protocol["jobs"]), False, 0
    with ThreadPoolExecutor(max_workers=protocol["workers"]) as pool:
        pending = {pool.submit(execute, root, protocol, next(jobs)) for _ in range(protocol["workers"])}
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
                committed_games=committed, at=time.time(), pid=os.getpid()))
            if not failed:
                for _ in finished:
                    job = next(jobs, None)
                    if job is not None:
                        pending.add(pool.submit(execute, root, protocol, job))
    write(root / "state.json", dict(state="failed" if failed else "complete_unverified", committed_games=committed, at=time.time()))
    if not failed:
        verify(root)
        write(root / "state.json", dict(state="complete_verified", committed_games=committed, at=time.time()))
    return not failed


def verify(root):
    protocol = validate_inputs(root)
    require(not list((root / "results").glob("*.rejected.json")), "rejected games preserved")
    require(len(list((root / "results").glob("*.done.json"))) == protocol["games"], "incomplete receipts")
    receipts = {}
    for job in protocol["jobs"]:
        receipt = read(root / f"results/{job}.done.json")
        require(receipt["job"] == job and receipt["manifest_sha256"] == digest(root / "manifest.json"), "receipt manifest")
        for key, folder in (("request", "requests"), ("output", "results")):
            require(receipt[key + "_sha256"] == digest(root / f"{folder}/{job}.json"), key + " receipt hash")
        request = read(root / f"requests/{job}.json")
        stats = validate_output(request, read(root / f"results/{job}.json"), read(root / request["plan"]), read(root / request["root_path"]))
        require(all(receipt[k] == v for k, v in stats.items()), "receipt semantics")
        receipts[job] = digest(root / f"results/{job}.done.json")
    proof = dict(verified_at=time.time(), games=protocol["games"], scored=protocol["scored"], platform=protocol["platform"],
        manifest_sha256=digest(root / "manifest.json"), receipts=receipts, full_launch_authorized=False)
    write(root / "completion-verified.json", proof)
    return proof


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=("prepare", "prepare-full", "run", "verify"))
    parser.add_argument("--root", type=Path, required=True)
    for name in ("original", "preregistration", "corpus", "runtime", "provenance", "repo"):
        parser.add_argument("--" + name, type=Path)
    parser.add_argument("--platform", choices=("windows", "linux"))
    for name in ("windows", "linux", "parity-path"):
        parser.add_argument("--" + name, type=Path)
    args = vars(parser.parse_args())
    command = args.pop("command")
    if command == "prepare":
        result = prepare(**{k:v for k,v in args.items() if k not in ("windows","linux","parity_path")})
    elif command == "prepare-full":
        result = prepare_full(**{k:args[k] for k in ("root","windows","linux","parity_path","preregistration","platform")})
    else:
        result = run(args["root"]) if command == "run" else verify(args["root"])
    print(json.dumps(result))
    if result is False:
        raise SystemExit(1)
