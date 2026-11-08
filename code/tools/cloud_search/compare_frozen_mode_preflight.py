"""Exact reference-game parity; no numerical tolerances or score pooling."""
import argparse
import copy
from pathlib import Path
import subprocess
import sys
from tools.cloud_search import frozen_mode_preflight as controller
from tools.cloud_search.compare_mulligan_continuations import differences


def comparable(output):
    value = copy.deepcopy(output)
    value.pop("runtime_sha256")
    value["request"].pop("runtime_sha256")
    for game in value["games"]:
        game.pop("seconds")
        for step in game["card_metrics"]["steps"]:
            step.pop("selection_seconds")
        for search in game["search_diagnostics"]:
            search["diagnostics"].pop("seconds")
            if search["diagnostics"]["information"]["regime"] == "closed-list":
                search["diagnostics"]["information"].pop("sampling_seconds")
    return value


def shared_manifest_inputs(manifest):
    return {key: value for key, value in manifest.items() if key.startswith("assets/") or key in
            ("controller.py", "frozen_mode_receipts.py", "inputs/schedule.json",
             "inputs/source-manifest.json", "inputs/train_snapshot.json", "inputs/heldout_snapshot.json")}


def verify_shared_inputs(windows, linux, manifests):
    """Keep exact input identities, with one recorded OS text-encoding case.

    Path.write_text emits CRLF on Windows and LF on Linux. Only the generated
    schedule may differ by that exact whole-file transformation. Both parsed
    schedules must also equal the complete frozen schedule; other input bytes,
    whitespace, order, values, and all game comparisons remain exact.
    """
    require, digest = controller.require, controller.digest
    left, right = map(shared_manifest_inputs, manifests)
    if left == right:
        return []
    name = "inputs/schedule.json"
    require(left.keys() == right.keys() and
            {key for key in left if left[key] != right[key]} == {name},
            "frozen inputs mismatch")
    wp, lp = windows / name, linux / name
    require(digest(wp) == left[name] and digest(lp) == right[name],
            "frozen schedule input changed")
    wb, lb = wp.read_bytes(), lp.read_bytes()
    require(b"\r" not in lb and wb == lb.replace(b"\n", b"\r\n"),
            "schedule differs beyond native CRLF/LF encoding")
    require(controller.read(wp) == controller.read(lp) == controller.schedule(),
            "encoded schedule coverage/seeds changed")
    return [dict(path=name, windows_sha256=left[name], linux_sha256=right[name],
                 canonical_lf_sha256=right[name], newline_count=lb.count(b"\n"),
                 transformation="Windows bytes exactly equal Linux LF bytes expanded to CRLF",
                 original_frozen_files_modified=False)]


def compare(windows, linux, out):
    require, read, digest = controller.require, controller.read, controller.digest
    require(not out.exists(), "preserve earlier parity proof")
    for root in (windows, linux):
        result = subprocess.run([sys.executable, str(root/"controller.py"), "verify", "--root", str(root)],
                                capture_output=True, text=True)
        require(result.returncode == 0, "frozen verification failed: " + result.stderr[-1500:])
    protocols = [read(r/"protocol.json") for r in (windows,linux)]
    require([p["platform"] for p in protocols] == ["windows","linux"], "platform identities")
    builds = [read(r/"inputs/build-provenance.json") for r in (windows,linux)]
    for key in ("portable_policy_math","features","source_manifest_sha256","dependency_lock_sha256"):
        require(builds[0][key] == builds[1][key], "build source/math mismatch: " + key)
    manifests = [read(r/"manifest.json")["files"] for r in (windows,linux)]
    encoding = verify_shared_inputs(windows, linux, manifests)
    all_differences, outputs = [], {}
    for job in controller.schedule():
        name=job["id"]
        requests=[read(r/f"requests/{name}.json") for r in (windows,linux)]
        plans=[read(r/q["plan"]) for r,q in zip((windows,linux),requests)]
        for plan in plans: plan.pop("native_sha256")
        require(plans[0] == plans[1], "plan settings differ")
        values=[read(r/f"results/{name}.json") for r in (windows,linux)]
        all_differences.extend(differences(*map(comparable,values),path=name))
        outputs[name]={host:digest(r/f"results/{name}.json") for host,r in (("windows",windows),("linux",linux))}
    proof=dict(schema="frozen-mode-preflight-parity-v1",passed=not all_differences,
        input_encoding_equivalence=encoding,
        requests_compared=134,games_compared=268,scored=False,differences=all_differences,outputs=outputs,
        manifests={host:digest(r/"manifest.json") for host,r in (("windows",windows),("linux",linux))},
        comparator_sha256=digest(Path(__file__)),
        ignored_fields=["runtime_sha256","request.runtime_sha256","games[].seconds",
            "games[].card_metrics.steps[].selection_seconds","games[].search_diagnostics[].diagnostics.seconds",
            "games[].search_diagnostics[].diagnostics.information.sampling_seconds"])
    controller.write(out,proof)
    return {"passed":proof["passed"],"games_compared":268,"differences":len(all_differences)}


if __name__=="__main__":
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ("windows","linux","out"):
        parser.add_argument("--"+name,type=Path,required=True)
    result=compare(**vars(parser.parse_args()))
    print(result)
    if not result["passed"]: raise SystemExit(1)
