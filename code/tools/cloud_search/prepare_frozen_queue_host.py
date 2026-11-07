"""Install one frozen psutil wheel into a NEW private host directory, no pip/system edits."""
import argparse
import hashlib
import json
from pathlib import Path
import sys
import time
import zipfile


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


if __name__ == '__main__':
    p=argparse.ArgumentParser()
    p.add_argument('--wheel',type=Path,required=True)
    p.add_argument('--sha256',required=True)
    p.add_argument('--root',type=Path,required=True)
    a=p.parse_args()
    if a.root.exists() or sha(a.wheel)!=a.sha256:
        raise ValueError('new private root and exact wheel hash required')
    with zipfile.ZipFile(a.wheel) as z:
        for name in z.namelist():
            if not (a.root/name).resolve().is_relative_to(a.root.resolve()):
                raise ValueError('wheel path escape')
        a.root.mkdir(parents=True)
        z.extractall(a.root)
    sys.path.insert(0,str(a.root))
    import psutil
    proof=dict(at=time.time(),python=sys.version,wheel_sha256=a.sha256,
               psutil_version=psutil.__version__,logical_cpus=psutil.cpu_count(),
               available_memory_bytes=psutil.virtual_memory().available,
               process_inventory_available=len(psutil.pids())>0,
               files={str(f.relative_to(a.root)):sha(f) for f in a.root.rglob('*')
                      if f.is_file() and '__pycache__' not in f.parts})
    (a.root/'dependency-verified.json').write_text(json.dumps(proof,indent=2)+'\n')
    print(json.dumps({k:v for k,v in proof.items() if k!='files'}))
