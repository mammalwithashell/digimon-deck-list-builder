"""Launch the approved diagnostic replay on the existing host, never training."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess


def run(args, **kwargs):
    return subprocess.run(args, check=True, **kwargs)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--image", required=True)
    parser.add_argument("--commit", required=True)
    parser.add_argument("--ci-run", required=True)
    parser.add_argument("--overlay", type=Path, required=True)
    parser.add_argument("--overlay-sha256", required=True)
    parser.add_argument("--overlay-commit", required=True)
    args = parser.parse_args()
    if "@sha256:" not in args.image:
        raise ValueError("An immutable image is required")
    if hashlib.sha256(args.overlay.read_bytes()).hexdigest() != args.overlay_sha256:
        raise ValueError("Overlay hash mismatch")
    revision = subprocess.check_output(["docker", "image", "inspect", args.image,
        "--format", '{{index .Config.Labels "org.opencontainers.image.revision"}}'], text=True).strip()
    if revision != args.commit:
        raise ValueError("Image source revision mismatch")
    for name in ("digimon-search", "digimon-seed-comparison", "digimon-mixed-continuation"):
        status = subprocess.run(["systemctl", "is-active", name + ".service"], capture_output=True, text=True).stdout.strip()
        if status in ("active", "activating", "deactivating"):
            raise ValueError(f"Training service is still running: {name}")
    base = Path("/opt/digimon-search")
    root = base / "decision-telemetry-eval-20261026"
    source = base / "final-policy-eval-20261026"
    root.mkdir(exist_ok=False)
    (root / "deployment.json").write_text(json.dumps(vars(args), default=str, indent=2))
    overlay_mount = f"type=bind,src={args.overlay},dst=/app/code/tools/cloud_search/decision_telemetry_eval.py,readonly"
    source_mount = f"type=bind,src={source},dst=/source,readonly"
    common = ["/usr/bin/docker", "run", "--rm", "--init",
              "-e", f"DIGIMON_DEPLOYED_IMAGE={args.image}",
              "--mount", overlay_mount, "--mount", source_mount]
    run(common + ["--network=none", "--memory=8g", "--cpus=2",
        "--mount", f"type=bind,src={root},dst=/work", args.image,
        "python", "-m", "tools.cloud_search.decision_telemetry_eval",
        "--prepare", "--source", "/source", "--root", "/work/evaluation",
        "--source-commit", args.commit, "--ci-run", args.ci_run])
    evaluation_mount = f"type=bind,src={root / 'evaluation'},dst=/evaluation"
    evaluate = common + ["--name", "digimon-decision-telemetry-eval", "--network=none",
        "--memory=16g", "--cpus=4", "--stop-timeout=120", "--mount", evaluation_mount,
        args.image, "python", "-m", "tools.cloud_search.final_policy_eval", "--root", "/evaluation"]
    publish = common + ["--name", "digimon-decision-telemetry-report", "--memory=4g", "--cpus=1",
        "--env-file", str(base / "monitor/credentials.env"), "--mount", evaluation_mount,
        args.image, "python", "-m", "tools.cloud_search.decision_telemetry_eval",
        "--source", "/source", "--root", "/evaluation", "--publish", "--wait",
        "--run-id", "decision-telemetry-eval-20261026"]
    units = []
    for suffix, command in [("eval", evaluate), ("report", publish)]:
        name = f"digimon-decision-telemetry-{suffix}.service"
        path = Path("/etc/systemd/system") / name
        # Fixed no-space paths; no shell interpretation or credential contents.
        text = "\n".join(["[Unit]", "Description=Digimon checkpoint decision diagnostics",
            "After=docker.service network-online.target", "[Service]", "Type=simple",
            "Restart=no", "RuntimeMaxSec=6h", "TimeoutStopSec=150",
            "ExecStart=" + " ".join(command), ""])
        with path.open("x") as stream:
            stream.write(text)
        units.append(name)
    run(["systemctl", "daemon-reload"])
    run(["systemctl", "start", *units])
    print(json.dumps(dict(status="launched", root=str(root), training=False, automatic_promotion=False)))


if __name__ == "__main__":
    main()
