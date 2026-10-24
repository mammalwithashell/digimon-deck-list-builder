"""Incrementally harvest committed training chunks and checkpoint archives over SSH.

Runs on the development host. Never deletes the server or remote artifacts.
"""
import argparse
import json
from pathlib import Path
import re
import shlex
import subprocess
import time

from tools.selfplay_scheduler import _read_commit, atomic_json, controller_lock, sha256


REMOTE_INDEX = r'''
from pathlib import Path
import hashlib,json,tarfile
root=Path('/opt/digimon-search/experiment')
chunks=sorted(str(p.parent.relative_to(root/'generation')) for p in (root/'generation').glob('gen*/shards/proc*/chunk_commit.json'))
archive=root.parent/'checkpoint-backup.tar.gz'
with tarfile.open(archive,'w:gz') as out:
    for p in root.rglob('*'):
        rel=p.relative_to(root)
        if not p.is_file() or p.is_symlink(): continue
        if 'attempts' in rel.parts or p.suffix=='.npy' or p.name.endswith('.tmp'): continue
        out.add(p,arcname=str(rel),recursive=False)
state=json.loads((root/'experiment_state.json').read_text()) if (root/'experiment_state.json').exists() else {'stage':'pending'}
print(json.dumps({'chunks':chunks,'state':state,'archive_sha256':hashlib.sha256(archive.read_bytes()).hexdigest()}))
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--host", required=True)
    parser.add_argument("--key", required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--watch", action="store_true")
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    ssh = ["ssh", "-i", args.key, "-o", "BatchMode=yes", "-o", "ConnectTimeout=15", args.host]
    scp = ["scp", "-q", "-i", args.key, "-o", "BatchMode=yes", "-o", "ConnectTimeout=15"]
    with controller_lock(args.out / "backup.lock"):
        while True:
            try:
                raw = subprocess.check_output(ssh + ["python3 -c " + shlex.quote(REMOTE_INDEX)], text=True, timeout=180)
                index = json.loads(raw)
                for relative in index["chunks"]:
                    if not re.fullmatch(r"gen\d+/shards/proc\d+", relative):
                        raise ValueError(f"Unexpected chunk path {relative}")
                    target = args.out / "generation" / relative
                    if target.exists():
                        continue
                    target.parent.mkdir(parents=True, exist_ok=True)
                    temporary = target.with_name(target.name + f".partial-{time.time_ns()}")
                    subprocess.run(scp + ["-r", f"{args.host}:/opt/digimon-search/experiment/generation/{relative}", str(temporary)], check=True, timeout=300)
                    saved = json.loads((temporary / "chunk_commit.json").read_text())
                    _read_commit(temporary, saved["game_start"], saved["games"], saved["seed"])
                    temporary.rename(target)
                temporary = args.out / "checkpoint-backup.tar.gz.partial"
                subprocess.run(scp + [f"{args.host}:/opt/digimon-search/checkpoint-backup.tar.gz", str(temporary)], check=True, timeout=300)
                if sha256(temporary) != index["archive_sha256"]:
                    raise ValueError("Checkpoint archive transfer hash mismatch")
                temporary.replace(args.out / "checkpoint-backup.tar.gz")
                atomic_json(args.out / "backup_status.json", {"time_unix": time.time(), **index})
                print(json.dumps({"time_unix": time.time(), "chunks_backed_up": len(index["chunks"]), "state": index["state"]}), flush=True)
                if not args.watch or index["state"]["stage"] in ("complete", "failed"):
                    return
            except Exception as error:
                print(f"Backup failed: {error}", flush=True)
                if not args.watch:
                    raise
            time.sleep(600)


if __name__ == "__main__":
    main()
