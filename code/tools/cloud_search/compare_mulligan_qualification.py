"""Exact cross-platform comparison of separately validated mulligan sweeps."""
import argparse
import importlib.util
from pathlib import Path
import time

from tools.cloud_search.mulligan_diagnostic_controller import digest, read, require, write


def qualified(root):
    # Verify using each campaign's own frozen controller, not mutable checkout code.
    spec = importlib.util.spec_from_file_location("frozen_mulligan_controller", root / "controller.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    module.verify(root)
    return read(root / "protocol.json")


def comparable(output):
    # Timing is the ONLY excluded native-output field. No tolerances or rounding.
    return [{k: v for k, v in row.items() if k != "seconds"} for row in output["rows"]]


def matching_build_sources(windows, linux):
    for key in ("base_commit", "archive_sha256", "overlays", "additional_overlay", "extra_archives"):
        require(windows[key] == linux[key], f"build source differs: {key}")
    # Optional only for historical proofs. New opt-in arithmetic must be equal
    # on both hosts, including its dependency lock, not merely equal Rust files.
    for key in ("features", "dependency_lock_sha256", "portable_math"):
        require(windows.get(key) == linux.get(key), f"build configuration differs: {key}")


def compare(windows, linux, out):
    require(not out.exists(), "preserve prior parity outcome")
    wp, lp = qualified(windows), qualified(linux)
    require(wp["platform"] == "windows" and lp["platform"] == "linux", "platform assignment")
    require(wp["jobs"] == lp["jobs"] and wp["roots"] == lp["roots"], "cohort/schedule mismatch")
    wm, lm = read(windows / "manifest.json"), read(linux / "manifest.json")
    comparable_paths = {p for p in wm["files"] if p.startswith(("assets/", "plans/", "requests/")) or p == "inputs/corpus.json"}
    require(all(wm["files"][p] == lm["files"].get(p) for p in comparable_paths), "input content differs")
    # Both binary builds must name exactly the same Rust overlays/base source.
    ws, ls = read(windows / "inputs/build-provenance.json"), read(linux / "inputs/build-provenance.json")
    matching_build_sources(ws, ls)
    differences, evidence = [], {}
    for job in wp["jobs"]:
        wo, lo = windows / f"results/{job}.json", linux / f"results/{job}.json"
        wr, lr = comparable(read(wo)), comparable(read(lo))
        for a, b in zip(wr, lr):
            if a != b:
                differences.append(dict(job=job, node=a["node"], fields=[k for k in a if a[k] != b.get(k)]))
        evidence[job] = dict(windows=digest(wo), linux=digest(lo))
    result = dict(verified_at=time.time(), passed=not differences, windows_manifest=digest(windows / "manifest.json"),
                  linux_manifest=digest(linux / "manifest.json"), sweep_outputs=evidence,
                  decisions_compared=wp["roots"]*len(wp["jobs"]), differences=differences,
                  only_ignored_fields=["seconds"], scope="known-list mulligan diagnostic; no closed-search qualification")
    write(out, result)
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--windows", type=Path, required=True)
    parser.add_argument("--linux", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    result = compare(args.windows, args.linux, args.out)
    print({"passed": result["passed"], "decisions_compared": result["decisions_compared"], "differences": len(result["differences"])})
    if not result["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
