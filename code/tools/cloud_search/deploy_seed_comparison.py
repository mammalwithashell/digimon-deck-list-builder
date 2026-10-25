"""Deploy the approved matched seed comparison on the existing CPU host.

Run on the host after image qualification. The failed baseline is read-only;
separate services never restart it or promote a candidate.
"""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import time


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--image", required=True)
    p.add_argument("--commit", required=True)
    p.add_argument("--ci-run", required=True)
    p.add_argument("--validation", type=Path, required=True)
    p.add_argument("--start", action="store_true")
    args = p.parse_args()
    if "@sha256:" not in args.image:
        raise ValueError("immutable image required")
    report = json.loads(args.validation.read_text())
    if (report.get("status") != "passed" or report.get("image") != args.image
            or report.get("source_commit") != args.commit
            or not any(c["label"] == "security-game268" and c["truncated_games"] == 0
                       for c in report.get("native", []))):
        raise ValueError("this image must pass the recorded search failure before deployment")
    inspect = json.loads(subprocess.check_output(["docker", "image", "inspect", args.image], text=True))[0]
    if inspect["Config"]["Labels"]["org.opencontainers.image.revision"] != args.commit:
        raise ValueError("source revision mismatch")
    for unit in ("digimon-search.service", "digimon-seed-comparison.service"):
        status = subprocess.run(["systemctl", "is-active", unit], capture_output=True, text=True).stdout.strip()
        if status in ("active", "activating", "deactivating"):
            raise RuntimeError(f"{unit} is still running")
    base = Path("/opt/digimon-search")
    root = base / "mixed-seed-comparison-20261025"
    source = base / "fresh-schema2-generations-20261025/inputs"
    plan = dict(image=args.image, source_commit=args.commit, ci_run=args.ci_run,
                validation=str(args.validation), baseline=str(source.parent), automatic_promotion=False)
    if not root.exists():
        (root / "inputs").mkdir(parents=True)
        for name in ("decks.json", "train_snapshot.json", "heldout_snapshot.json", "pet_evaluation_snapshot.json"):
            shutil.copy2(source / name, root / "inputs" / name)
        (root / "host-plan.json").write_text(json.dumps(plan, indent=2))
    if json.loads((root / "host-plan.json").read_text()) != plan or (root / "started.json").exists():
        raise ValueError("preserve existing experiment; plan differs or it already started")
    if not args.start:
        print(json.dumps(dict(status="prepared", root=str(root), plan=plan)))
        return
    units = Path("/etc/systemd/system")
    def unit(name, command):
        lines = ["[Unit]", "Description=Digimon matched seed experiment", "After=docker.service network-online.target",
                 "[Service]", "Type=simple", "Restart=no", "RuntimeMaxSec=48h", "TimeoutStopSec=150"]
        lines += ["ExecStart=" + " ".join(command), "[Install]", "WantedBy=multi-user.target", ""]
        (units / name).write_text("\n".join(lines))
    run = ["/usr/bin/docker", "run", "--rm", "--init", "--name", "digimon-seed-comparison",
           "--network=none", "--memory=28g", "--cpus=8", "--stop-timeout=120",
           "-e", f"DIGIMON_DEPLOYED_IMAGE={args.image}",
           "--mount", f"type=bind,src={root},dst=/experiment", args.image,
           "python", "-m", "tools.cloud_search.run_seed_comparison"]
    unit("digimon-seed-comparison.service", run)
    for arm in ("greedy", "development", "mixed"):
        state = root / "monitors" / arm
        state.mkdir(parents=True, exist_ok=False)
        logging = ["/usr/bin/docker", "run", "--rm", "--init", "--name", f"digimon-seed-monitor-{arm}",
                   "--memory=1g", "--cpus=0.25", "--cpu-shares=128",
                   "--env-file", str(base / "monitor/credentials.env"),
                   "--mount", f"type=bind,src={root},dst=/experiment,readonly",
                   "--mount", f"type=bind,src={state},dst=/monitor", args.image,
                   "python", "-m", "tools.cloud_search.wandb_monitor",
                   "--root", f"/experiment/arms/{arm}", "--training-log", f"/experiment/arms/{arm}/training.log",
                   "--card-eval-root", f"/experiment/evaluations/{arm}",
                   "--run-id", f"english-meta-seed-{arm}-20261025", "--online"]
        unit(f"digimon-seed-monitor-{arm}.service", logging)
    subprocess.run(["systemctl", "daemon-reload"], check=True)
    subprocess.run(["systemctl", "start", "digimon-seed-comparison.service"], check=True)
    for _ in range(60):
        if all((root / "arms" / arm / "deployment-plan.json").exists() for arm in ("greedy", "development", "mixed")):
            break
        time.sleep(1)
    else:
        raise RuntimeError("controller did not prepare arms; inspect its service before starting monitors")
    for arm in ("greedy", "development", "mixed"):
        subprocess.run(["systemctl", "start", f"digimon-seed-monitor-{arm}.service"], check=True)
    print(json.dumps(dict(status="launched", root=str(root), plan=plan, time_unix=time.time())))


if __name__ == "__main__":
    main()
