"""Exact eight-game qualification comparison; only clock/runtime identities differ."""
import argparse
import copy
import json
from pathlib import Path
from tools.cloud_search import mulligan_continuation_controller as controller


def comparable(output):
    value = copy.deepcopy(output)
    # Each platform must already bind its own verified executable and plan.
    for container in (value, value["request"]):
        for key in ("runtime_sha256", "plan_sha256"):
            container.pop(key)
    value["game"].pop("seconds")
    for step in value["game"]["card_metrics"]["steps"]:
        step.pop("selection_seconds")
    return value


def differences(left, right, path=""):
    if type(left) is not type(right):
        return [path + " (type)"]
    if isinstance(left, dict):
        if left.keys() != right.keys():
            return [path + " (keys)"]
        return [p for key in left for p in differences(left[key], right[key], path + "/" + str(key))]
    if isinstance(left, list):
        if len(left) != len(right):
            return [path + " (length)"]
        return [p for index, (a, b) in enumerate(zip(left, right)) for p in differences(a, b, path + f"/{index}")]
    return [] if left == right else [path]


def compare(windows, linux, out):
    require = controller.require
    require(not out.exists(), "preserve prior comparison")
    for root in (windows, linux):
        controller.verify(root)
    wp, lp = (controller.read(r / "protocol.json") for r in (windows, linux))
    require(wp["platform"] == "windows" and lp["platform"] == "linux", "platform identities")
    require(wp["jobs"] == lp["jobs"] and len(wp["jobs"]) == 8, "same eight excluded games")
    provenance = [controller.read(r / "inputs/build-provenance.json") for r in (windows, linux)]
    for key in ("portable_policy_math", "features", "source_files", "dependency_lock_sha256", "source_manifest_sha256"):
        require(provenance[0][key] == provenance[1][key], "build provenance: " + key)
    manifests = [controller.read(r / "manifest.json")["files"] for r in (windows, linux)]
    shared = lambda m: {k:v for k,v in m.items() if k.startswith("assets/") or k in
                        ("controller.py", "inputs/root.json", "inputs/scored-roots.json", "inputs/preregistered-protocol.json")}
    require(shared(manifests[0]) == shared(manifests[1]), "shared frozen inputs differ")
    all_differences, outputs = [], {}
    for job in wp["jobs"]:
        requests = [controller.read(r / f"requests/{job}.json") for r in (windows, linux)]
        plans = [controller.read(r / q["plan"]) for r, q in zip((windows, linux), requests)]
        for plan in plans:
            plan.pop("runtime_sha256")
        require(plans[0] == plans[1], "plan settings differ")
        values = [controller.read(r / f"results/{job}.json") for r in (windows, linux)]
        all_differences.extend(differences(*map(comparable, values), path=job))
        outputs[job] = {host:controller.digest(r / f"results/{job}.json") for host,r in (("windows",windows),("linux",linux))}
    proof = dict(schema="mulligan-continuation-parity-v1", passed=not all_differences,
        games_compared=8, scored=False, differences=all_differences,
        qualification_manifests={host:controller.digest(r / "manifest.json") for host,r in (("windows",windows),("linux",linux))},
        outputs=outputs, comparator_sha256=controller.digest(Path(__file__)),
        ignored_fields=["runtime_sha256", "plan_sha256", "game.seconds", "game.card_metrics.steps[].selection_seconds"])
    controller.write(out, proof)
    return proof


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    for name in ("windows", "linux", "out"):
        parser.add_argument("--"+name, required=True, type=Path)
    proof = compare(**vars(parser.parse_args()))
    print(json.dumps({k:proof[k] for k in ("passed", "games_compared")}|{"differences":len(proof["differences"])}))
    if not proof["passed"]:
        raise SystemExit(1)
