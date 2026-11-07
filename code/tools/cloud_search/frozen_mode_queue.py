"""Single-attempt, host-coordinated queues of already-qualified frozen panels.

Run this controller on the host, not inside a PID-isolated container. Linux
panels run in retained, network-disabled containers; Windows panels run native.
Only one queue holds the shared host lock at a time. Other workloads are never
stopped. A capacity wait does not consume a panel attempt.
"""
import argparse
from contextlib import contextmanager
import hashlib
import json
import math
import os
from pathlib import Path, PurePosixPath, PureWindowsPath
import shutil
import subprocess
import tarfile
import sys
import time

try:
    from . import frozen_mode_campaign as campaign
except ImportError:
    import panel_validation as campaign

read, write, digest, require = campaign.read, campaign.write, campaign.digest, campaign.require
IMAGE = 'ghcr.io/mammalwithashell/digimon-search-cpu@sha256:e550a7473f65a4d1a0ddd18b8d43b486cc64236cc58c935578a0169c4b07e33e'
GIB = 1024**3


def container_memory_floor(limits):
    value = limits.get('container_memory_floor_bytes', 0)
    require(type(value) is int and value >= 0, 'nonnegative integer container memory floor required')
    return value


def container_memory_limit(limits, workers, peak):
    return max(math.ceil(peak*workers*1.5) + GIB, container_memory_floor(limits))


def capacity_reasons(snapshot, limits, workers, peak, artifact_bytes):
    """Conservative admission from live host evidence, never from heartbeat age."""
    require(1 <= workers <= 4 and peak > 0 and artifact_bytes > 0, 'allocation estimates')
    require(snapshot['logical_cpus'] > 0 and 0 <= snapshot['cpu_percent'] <= 100, 'CPU evidence')
    reasons = []
    if snapshot['native_workers'] + workers > limits['native_cap']:
        reasons.append('native worker cap')
    if snapshot['build_processes']:
        reasons.append('active native build')
    if snapshot['cpu_percent'] + 100*workers/snapshot['logical_cpus'] > limits['cpu_ceiling_percent']:
        reasons.append('CPU headroom')
    memory = math.ceil(peak*workers*1.5)
    if container_memory_floor(limits):
        memory = container_memory_limit(limits, workers, peak)
    if snapshot['available_memory_bytes'] < memory + limits['memory_reserve_bytes']:
        reasons.append('memory headroom')
    if snapshot['disk_free_bytes'] < artifact_bytes + limits['disk_reserve_bytes']:
        reasons.append('artifact disk headroom')
    return reasons


def probe(root):
    import psutil  # Host dependency; fail closed if absent. Never install during a run.
    require(not Path('/.dockerenv').exists(), 'queue must run on host for cross-workload visibility')
    processes = []
    native = builds = 0
    for proc in psutil.process_iter(['pid', 'name', 'cmdline']):
        try:
            info = proc.info
            command = info.get('cmdline') or []
            name = Path(command[0]).name.lower() if command else (info.get('name') or '').lower()
            is_build = name in ('cargo', 'cargo.exe', 'rustc', 'rustc.exe')
            is_native = (name.startswith(('frozen_checkpoint_modes', 'mulligan_continuation',
                'mulligan_search_diagnostic', 'checkpoint_search_eval', 'digimon-selfplay', 'digimon_selfplay')))
            native += int(is_native); builds += int(is_build)
            if is_build or is_native:
                processes.append(dict(pid=info['pid'], name=name, command=command))
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            continue
    return dict(at=time.time(), logical_cpus=psutil.cpu_count(), cpu_percent=psutil.cpu_percent(interval=1),
        available_memory_bytes=psutil.virtual_memory().available, disk_free_bytes=shutil.disk_usage(root).free,
        native_workers=native, build_processes=builds, processes=processes,
        psutil_version=psutil.__version__, scope='Host-wide load/memory; exact recognized native/build commands')


@contextmanager
def host_lock(path):
    """One new-program queue per host; process death releases the OS lock."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('a+b') as stream:
        stream.seek(0, os.SEEK_END)
        if stream.tell() == 0:
            stream.write(b'0'); stream.flush()
        stream.seek(0)
        if os.name == 'nt':
            import msvcrt
            msvcrt.locking(stream.fileno(), msvcrt.LK_NBLCK, 1)
        else:
            import fcntl
            fcntl.flock(stream.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
        try:
            yield
        finally:
            stream.seek(0)
            if os.name == 'nt':
                msvcrt.locking(stream.fileno(), msvcrt.LK_UNLCK, 1)
            else:
                fcntl.flock(stream.fileno(), fcntl.LOCK_UN)


def normalized_lock(shared_lock, platform):
    """Validate the destination OS path even when preparation runs on another OS."""
    value=str(shared_lock)
    if platform=='linux':
        value=value.replace('\\','/')
        path=PurePosixPath(value)
        require(path.is_absolute() and not PureWindowsPath(value).drive, 'absolute Linux host lock required')
        return path.as_posix()
    require(platform=='windows' and PureWindowsPath(value).is_absolute(), 'absolute Windows host lock required')
    return PureWindowsPath(value).as_posix()


def prepare(root, panels, artifact_bytes_per_panel, shared_lock, container_memory_floor_bytes=0):
    require(not root.exists() and panels, 'new queue and nonempty panel list required')
    require(artifact_bytes_per_panel > 0 and shared_lock is not None, 'artifact estimate/shared host lock')
    floor = container_memory_floor(dict(container_memory_floor_bytes=container_memory_floor_bytes))
    identities = []; platform = schedule = None
    for panel_root in panels:
        protocol, _, _ = campaign.validate_inputs(panel_root)
        require(not (panel_root/'started.json').exists() and not any((panel_root/'attempts').iterdir())
            and not any((panel_root/'results').iterdir()), 'panel already attempted')
        platform = platform or protocol['platform']; schedule = schedule or protocol['schedule_manifest_sha256']
        require(platform == protocol['platform'] and schedule == protocol['schedule_manifest_sha256'], 'queue host/schedule mismatch')
        require(protocol['panel'] not in [p['id'] for p in identities], 'duplicate panel')
        identities.append(dict(id=protocol['panel'], root='panels/'+protocol['panel'],
            manifest_sha256=digest(panel_root/'manifest.json'), workers=protocol['workers']))
    lock=normalized_lock(shared_lock,platform)
    require(platform == 'linux' or floor == 0, 'container memory floor requires Linux')
    required_disk = sum(p.stat().st_size for panel in panels for p in panel.rglob('*') if p.is_file())
    require(shutil.disk_usage(root.parent).free > required_disk + artifact_bytes_per_panel + 2*GIB, 'preparation disk headroom')
    root.mkdir()
    for name in ('panels', 'attempts', 'logs', 'capacity'):
        (root/name).mkdir()
    for panel_root, identity in zip(panels, identities):
        shutil.copytree(panel_root, root/identity['root'])
    # Never name this queue.py: that shadows Python's stdlib queue module,
    # which concurrent.futures imports while loading the frozen panel verifier.
    shutil.copy2(__file__, root/'host_controller.py')
    shutil.copy2(campaign.__file__, root/'panel_validation.py')
    shutil.copy2(Path(campaign.receipts.__file__), root/'frozen_mode_receipts.py')
    import inspect
    shutil.copy2(inspect.getsourcefile(campaign.validate_blocks), root/'schedule_validation.py')
    protocol = dict(schema='frozen-mode-host-queue-v1', platform=platform, panels=identities,
        schedule_manifest_sha256=schedule, image=IMAGE if platform=='linux' else None,
        shared_host_lock=lock, artifact_bytes_per_panel=artifact_bytes_per_panel,
        limits=dict(native_cap=3 if platform=='windows' else 8, cpu_ceiling_percent=90,
            memory_reserve_bytes=2*GIB, disk_reserve_bytes=2*GIB,
            container_memory_floor_bytes=floor), capacity_poll_seconds=30,
        failure_policy='stop queue; preserve panel and in-flight failure; never resume/retry', no_retry=True)
    write(root/'protocol.json', protocol)
    write(root/'manifest.json', dict(files={p.name:digest(p) for p in root.iterdir() if p.is_file()}))
    return dict(state='prepared', panels=len(panels), launched=False)


def validate(root):
    campaign.frozen(root); protocol=read(root/'protocol.json')
    require(protocol['schema']=='frozen-mode-host-queue-v1' and protocol['no_retry'] is True, 'queue identity')
    require(protocol['image']==(IMAGE if protocol['platform']=='linux' else None), 'frozen evaluation image')
    floor = container_memory_floor(protocol['limits'])
    require(protocol['platform'] == 'linux' or floor == 0, 'container memory floor requires Linux')
    require(normalized_lock(protocol['shared_host_lock'],protocol['platform'])==protocol['shared_host_lock'], 'host lock path changed')
    ids=[]
    for panel in protocol['panels']:
        path=(root/panel['root']).resolve()
        require(path.is_relative_to(root.resolve()), 'panel path escape')
        require(digest(path/'manifest.json')==panel['manifest_sha256'], 'panel identity changed')
        p,_,_=campaign.validate_inputs(path)
        require(p['platform']==protocol['platform'] and p['workers']==panel['workers']
            and p['panel']==panel['id'] and p['schedule_manifest_sha256']==protocol['schedule_manifest_sha256'], 'panel allocation identity')
        ids.append(panel['id'])
    require(len(ids)==len(set(ids))>0, 'duplicate/empty queue')
    return protocol


def archive(root, destination):
    """Package an unattempted Linux queue without losing executable bits or empty dirs."""
    protocol=validate(root)
    require(protocol['platform']=='linux', 'deployment archive is for Linux queues')
    require(not destination.exists() and not destination.resolve().is_relative_to(root.resolve()), 'new archive outside queue required')
    require(not (root/'started.json').exists() and not any((root/'attempts').iterdir()), 'queue already attempted')
    executables=set();directories={'attempts','capacity','logs','panels'}
    for item in protocol['panels']:
        panel=root/item['root'];p=read(panel/'protocol.json')
        require(not (panel/'started.json').exists() and not any((panel/'attempts').iterdir())
                and not any((panel/'results').iterdir()), 'panel already attempted')
        executables.add(item['root']+'/'+p['binary'])
        directories.update(item['root']+'/'+name for name in ('attempts','logs','results'))
    def mode(info):
        require(not info.issym() and not info.islnk(), 'deployment symlinks are unsupported')
        if info.name in executables:info.mode=0o755
        return info
    with tarfile.open(destination,'w:gz') as bundle:
        for path in sorted(root.iterdir()):bundle.add(path,arcname=path.name,filter=mode)
    with tarfile.open(destination) as bundle:
        require(all(bundle.getmember(name).mode & 0o111 for name in executables), 'native executable mode missing')
        require(all(bundle.getmember(name).isdir() for name in directories), 'empty queue directories missing')
    return dict(archive_sha256=digest(destination),queue_manifest_sha256=digest(root/'manifest.json'),
                native_executables=len(executables),launched=False)


def command(root, protocol, item, index):
    panel=(root/item['root']).resolve()
    if protocol['platform']=='windows':
        return [sys.executable,str(panel/'controller.py'),'run','--root',str(panel)], None
    peak=read(panel/'inputs/profile.json')['observed_peak_worker_rss_bytes']
    memory=container_memory_limit(protocol.get('limits', {}),item['workers'],peak)
    name='frozen-eval-'+digest(root/'manifest.json')[:16]+'-'+str(index)
    return ['docker','run','--name',name,'--network','none','--cpus',str(item['workers']),
        '--memory',str(memory),'--mount',f'type=bind,src={panel},dst=/campaign',
        '--workdir','/campaign','--entrypoint','python3',protocol['image'],
        '/campaign/controller.py','run','--root','/campaign'], name


def run(root, snapshot=probe, execute=subprocess.run, pause=time.sleep):
    protocol=validate(root)
    require(not (root/'started.json').exists(), 'queue cannot resume')
    # Confirm host dependencies/visibility before consuming the one-shot queue identity.
    snapshot(root)
    with host_lock(Path(protocol['shared_host_lock'])):
        with (root/'started.json').open('x') as stream:
            json.dump(dict(at=time.time(),pid=os.getpid()),stream)
        try:
            for index,item in enumerate(protocol['panels']):
                panel=root/item['root']; peak=read(panel/'inputs/profile.json')['observed_peak_worker_rss_bytes']
                require(not (panel/'started.json').exists(), 'panel already launched')
                while True:
                    evidence=snapshot(root)
                    reasons=capacity_reasons(evidence,protocol['limits'],item['workers'],peak,protocol['artifact_bytes_per_panel'])
                    write(root/f'capacity/{index}-latest.json', dict(snapshot=evidence,waiting_reasons=reasons))
                    if not reasons: break
                    write(root/'state.json',dict(state='waiting_capacity',next_panel=item['id'],reasons=reasons,at=time.time()))
                    pause(protocol['capacity_poll_seconds'])
                campaign.validate_inputs(panel)
                cmd,container=command(root,protocol,item,index)
                # Durable host-wide claim also prevents another queue from copying
                # and replaying the same scheduled panel after this queue ends.
                claims=Path(protocol['shared_host_lock']).parent/'panel-claims'
                claims.mkdir(exist_ok=True)
                key=hashlib.sha256((protocol['schedule_manifest_sha256']+':'+item['id']).encode()).hexdigest()
                with (claims/(key+'.json')).open('x') as stream:
                    json.dump(dict(at=time.time(),queue=str(root),queue_manifest_sha256=digest(root/'manifest.json'),panel=item),stream)
                with (root/f'attempts/{index}.json').open('x') as stream:
                    json.dump(dict(at=time.time(),panel=item,command=cmd,capacity=evidence),stream)
                write(root/'state.json',dict(state='running',panel=item['id'],at=time.time()))
                flags=subprocess.CREATE_NO_WINDOW if os.name=='nt' else 0
                with (root/f'logs/{index}.out.log').open('xb') as out,(root/f'logs/{index}.err.log').open('xb') as err:
                    execute(cmd,check=True,stdout=out,stderr=err,creationflags=flags)
                if container:
                    result=execute(['docker','inspect','--format','{{json .State}}',container],check=True,capture_output=True,text=True)
                    terminal=json.loads(result.stdout)
                    write(root/f'capacity/{index}-terminal.json',terminal)
                    require(terminal['ExitCode']==0 and terminal['OOMKilled'] is False and not terminal['Running'], 'container terminal failure')
                # Platform-independent verification invokes the exact frozen panel controller.
                with (root/f'logs/{index}.verify.log').open('xb') as out:
                    execute([sys.executable,str(panel/'controller.py'),'verify','--root',str(panel.resolve())],check=True,stdout=out,stderr=out,creationflags=flags)
                complete=read(panel/'completion-verified.json')
                require(complete['complete'] is True and complete['manifest_sha256']==item['manifest_sha256'], 'panel completion proof')
                write(root/f'attempts/{index}.complete.json',dict(at=time.time(),panel=item['id'],proof_sha256=digest(panel/'completion-verified.json')))
            write(root/'state.json',dict(state='complete_verified',panels=len(protocol['panels']),at=time.time()))
        except Exception as error:
            write(root/'state.json',dict(state='failed',error=str(error),at=time.time(),no_retry=True))
            raise


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command',choices=('prepare','run','validate','probe','archive'))
    parser.add_argument('--root',type=Path,required=True)
    parser.add_argument('--panels',type=Path,nargs='+')
    parser.add_argument('--artifact-bytes-per-panel',type=int)
    parser.add_argument('--shared-lock',type=Path)
    parser.add_argument('--container-memory-floor-bytes',type=int,default=0)
    parser.add_argument('--out',type=Path)
    args=parser.parse_args();root=args.root.resolve()
    if args.command=='prepare': result=prepare(root,args.panels,args.artifact_bytes_per_panel,args.shared_lock,args.container_memory_floor_bytes)
    elif args.command=='run': result=run(root)
    elif args.command=='probe': result=probe(root)
    elif args.command=='archive': result=archive(root,args.out)
    else: result=validate(root)
    print(json.dumps(result))
