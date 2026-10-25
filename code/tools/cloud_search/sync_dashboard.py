"""Pull compact read-only snapshots from an SSH host or local experiment."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shlex
import subprocess
import sys
import time

from tools.cloud_search.dashboard_snapshot import collect, merge_snapshot, read
from tools.selfplay_scheduler import controller_lock


def atomic(path, payload):
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(payload), encoding="utf-8")
    temporary.replace(path)


def synchronize(config, output):
    from tools.cloud_search import dashboard_snapshot
    previous = read(output / "snapshot.json", {})
    # Only acknowledge data actually cached, so failed reads are retried.
    known = {r["id"]: r["signature"] for r in previous.get("reports", [])}
    cards_path = Path(config.get("cards_json", Path(__file__).resolve().parents[3] / "data/cards.json"))
    catalog = {}
    if cards_path.exists():
        raw = cards_path.read_bytes()
        cards = json.loads(raw)
        # Windows checkouts may use CRLF while the pinned Linux image uses Git
        # LF bytes. Accept only these byte-identical-content representations.
        normalized = raw.replace(b"\r\n", b"\n")
        catalog = dict(cards_sha256=hashlib.sha256(normalized).hexdigest(),
                       cards_sha256_candidates=[hashlib.sha256(raw).hexdigest(), hashlib.sha256(normalized).hexdigest()],
                       card_ids=[cid for cid, row in cards.items() if row.get("dna_costs")])
    if previous.get("dna_catalog_hash") != catalog.get("cards_sha256"):
        known = {}  # eligibility changes invalidate the compact report cache
    if config.get("host"):
        host = config["host"]
        if host.startswith("-") or not re.fullmatch(r"[A-Za-z0-9_.@-]+", host):
            raise ValueError("Invalid SSH host")
        command = ["ssh", "-o", "BatchMode=yes", "-o", "ConnectTimeout=10"]
        if config.get("identity_file"):
            command += ["-i", str(Path(config["identity_file"]).expanduser())]
        remote = shlex.join(["python3", "-", "--root", config["root"],
                             "--known-reports", json.dumps(known, separators=(",", ":")),
                             "--dna-catalog", json.dumps(catalog, separators=(",", ":"))])
        result = subprocess.run(command + [host, remote],
            input=Path(dashboard_snapshot.__file__).read_text(encoding="utf-8"),
            text=True, capture_output=True, timeout=120, check=True)
        incoming = json.loads(result.stdout)
    else:
        incoming = collect(Path(config["root"]), known, dna_catalog=catalog)
    incoming["dna_catalog_hash"] = catalog.get("cards_sha256")
    merged = merge_snapshot(previous, incoming)
    atomic(output / "snapshot.json", merged)
    atomic(output / "sync_status.json", dict(status="ok", last_success=time.time(),
        reports=len(merged["reports"]), changed_reports=len(incoming["reports"])))
    return merged


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--once", action="store_true")
    parser.add_argument("--interval", type=int, default=60)
    args = parser.parse_args()
    if args.interval < 30:
        parser.error("Use an interval of at least 30 seconds")
    config = json.loads(args.config.read_text(encoding="utf-8-sig"))
    args.out.mkdir(parents=True, exist_ok=True)
    with controller_lock(args.out / "sync.lock"):
        run_loop(args, config)


def run_loop(args, config):
    while True:
        try:
            snapshot = synchronize(config, args.out)
            print(json.dumps(dict(status="ok", stage=snapshot["state"], reports=len(snapshot["reports"]))), flush=True)
        except (OSError, ValueError, subprocess.SubprocessError) as error:
            status = read(args.out / "sync_status.json", {})
            status.update(status="error", failed_at=time.time(), error=str(error))
            atomic(args.out / "sync_status.json", status)
            print(f"Snapshot refresh failed: {error}", file=sys.stderr, flush=True)
            if args.once:
                raise SystemExit(1)
        if args.once:
            break
        time.sleep(args.interval)


if __name__ == "__main__":
    main()
