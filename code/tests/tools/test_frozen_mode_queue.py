import json
from pathlib import Path
import subprocess
from types import SimpleNamespace

import pytest
from tools.cloud_search import frozen_mode_queue as q


LIMITS=dict(native_cap=3,cpu_ceiling_percent=90,memory_reserve_bytes=2*q.GIB,disk_reserve_bytes=2*q.GIB)


def available(**changes):
    value=dict(logical_cpus=16,cpu_percent=5,available_memory_bytes=16*q.GIB,
        disk_free_bytes=20*q.GIB,native_workers=0,build_processes=0)
    value.update(changes)
    return value


@pytest.mark.parametrize('change,reason',[
    ({'native_workers':1},'native worker cap'),
    ({'build_processes':1},'active native build'),
    ({'cpu_percent':80},'CPU headroom'),
    ({'available_memory_bytes':3*q.GIB},'memory headroom'),
    ({'disk_free_bytes':2*q.GIB},'artifact disk headroom'),
])
def test_waits_for_real_capacity(change,reason):
    assert reason in q.capacity_reasons(available(**change),LIMITS,3,q.GIB,q.GIB)


def test_capacity_requires_positive_frozen_estimates():
    assert q.capacity_reasons(available(),LIMITS,3,q.GIB,q.GIB)==[]
    for peak,artifact in [(0,q.GIB),(q.GIB,0)]:
        with pytest.raises(ValueError):q.capacity_reasons(available(),LIMITS,3,peak,artifact)


def fixture(tmp_path,monkeypatch,platform='windows'):
    root=tmp_path/'queue';root.mkdir()
    for name in ['capacity','attempts','logs','panels']:(root/name).mkdir()
    items=[]
    for name in ['a-cp12-vs-cp8','b-cp12-vs-cp8']:
        panel=root/'panels'/name;(panel/'inputs').mkdir(parents=True)
        q.write(panel/'inputs/profile.json',dict(observed_peak_worker_rss_bytes=q.GIB))
        q.write(panel/'manifest.json',{'files':{}})
        (panel/'controller.py').write_text('# inert test controller')
        items.append(dict(id=name,root='panels/'+name,manifest_sha256=q.digest(panel/'manifest.json'),workers=3))
    protocol=dict(platform=platform,panels=items,limits=LIMITS,artifact_bytes_per_panel=q.GIB,
        shared_host_lock=str(tmp_path/'host.lock'),capacity_poll_seconds=30,image=q.IMAGE if platform=='linux' else None,
        schedule_manifest_sha256='fixture-schedule')
    q.write(root/'manifest.json',{'files':{}})
    monkeypatch.setattr(q,'validate',lambda _:protocol)
    monkeypatch.setattr(q.campaign,'validate_inputs',lambda _:None)
    return root,protocol


def fake_executor(root, calls, failure=False):
    def execute(command,**kwargs):
        calls.append(command)
        if failure:raise subprocess.CalledProcessError(1,command)
        if command[0]=='docker' and command[1]=='inspect':
            return SimpleNamespace(stdout=json.dumps(dict(ExitCode=0,OOMKilled=False,Running=False)))
        if command[0]=='docker':
            mount=command[command.index('--mount')+1];panel=Path(mount.split('src=')[1].split(',dst=')[0])
        else:panel=Path(command[1]).parent
        if 'run' in command:
            (panel/'started.json').write_text('{}')
        else:
            q.write(panel/'completion-verified.json',dict(complete=True,manifest_sha256=q.digest(panel/'manifest.json')))
        return SimpleNamespace(returncode=0)
    return execute


def test_capacity_wait_does_not_consume_panel_attempt(tmp_path,monkeypatch):
    root,_=fixture(tmp_path,monkeypatch);calls=[];snapshots=iter([available(),available(native_workers=3),available(),available()])
    waits=[]
    def pause(seconds):
        assert list((root/'attempts').iterdir())==[]
        waits.append(seconds)
    q.run(root,snapshot=lambda _:next(snapshots),execute=fake_executor(root,calls),pause=pause)
    assert waits==[30] and len(calls)==4
    assert q.read(root/'state.json')['state']=='complete_verified'


def test_failure_stops_later_panels_and_forbids_resume(tmp_path,monkeypatch):
    root,_=fixture(tmp_path,monkeypatch);calls=[]
    with pytest.raises(subprocess.CalledProcessError):
        q.run(root,snapshot=lambda _:available(),execute=fake_executor(root,calls,True))
    assert len(calls)==1 and not (root/'attempts/1.json').exists()
    assert q.read(root/'state.json')['state']=='failed'
    with pytest.raises(ValueError,match='cannot resume'):
        q.run(root,snapshot=lambda _:available(),execute=fake_executor(root,calls))
    assert len(calls)==1


def test_frozen_verifier_failure_stops_queue(tmp_path,monkeypatch):
    root,_=fixture(tmp_path,monkeypatch);calls=[];success=fake_executor(root,calls)
    def execute(cmd,**kwargs):
        if 'verify' in cmd:raise subprocess.CalledProcessError(1,cmd)
        return success(cmd,**kwargs)
    with pytest.raises(subprocess.CalledProcessError):q.run(root,snapshot=lambda _:available(),execute=execute)
    assert not (root/'attempts/1.json').exists()


def test_linux_retains_network_disabled_immutable_container(tmp_path,monkeypatch):
    root,p=fixture(tmp_path,monkeypatch,'linux');calls=[]
    q.run(root,snapshot=lambda _:available(),execute=fake_executor(root,calls))
    cmd=calls[0]
    assert '--rm' not in cmd and cmd[cmd.index('--network')+1]=='none' and q.IMAGE in cmd
    assert cmd[cmd.index('--cpus')+1]=='3' and int(cmd[cmd.index('--memory')+1])>3*q.GIB
    assert (root/'capacity/0-terminal.json').exists()
    assert len(calls)==6


def test_oom_blocks_next_panel(tmp_path,monkeypatch):
    root,_=fixture(tmp_path,monkeypatch,'linux');calls=[];success=fake_executor(root,calls)
    def execute(cmd,**kwargs):
        if cmd[:2]==['docker','inspect']:
            return SimpleNamespace(stdout=json.dumps(dict(ExitCode=0,OOMKilled=True,Running=False)))
        return success(cmd,**kwargs)
    with pytest.raises(ValueError,match='terminal failure'):q.run(root,snapshot=lambda _:available(),execute=execute)
    assert not (root/'attempts/1.json').exists()


def test_preexisting_panel_cannot_launch(tmp_path,monkeypatch):
    root,p=fixture(tmp_path,monkeypatch);(root/p['panels'][0]['root']/'started.json').write_text('{}')
    calls=[]
    with pytest.raises(ValueError,match='already launched'):q.run(root,snapshot=lambda _:available(),execute=fake_executor(root,calls))
    assert calls==[]


def test_host_lock_prevents_competing_queue(tmp_path):
    path=tmp_path/'host.lock'
    with q.host_lock(path):
        with pytest.raises(OSError):
            with q.host_lock(path):pytest.fail('second queue acquired host lock')
    with q.host_lock(path):pass


def test_missing_probe_dependency_leaves_queue_unattempted(tmp_path,monkeypatch):
    root,_=fixture(tmp_path,monkeypatch)
    def missing(_):raise ImportError('psutil')
    with pytest.raises(ImportError):q.run(root,snapshot=missing)
    assert not (root/'started.json').exists()


def test_duplicate_host_panel_claim_blocks_native_invocation(tmp_path,monkeypatch):
    root,p=fixture(tmp_path,monkeypatch);claims=tmp_path/'panel-claims';claims.mkdir()
    key=q.hashlib.sha256((p['schedule_manifest_sha256']+':'+p['panels'][0]['id']).encode()).hexdigest()
    (claims/(key+'.json')).write_text('{"preserved_previous_attempt":true}')
    calls=[]
    with pytest.raises(FileExistsError):q.run(root,snapshot=lambda _:available(),execute=fake_executor(root,calls))
    assert calls==[] and not (root/'attempts/0.json').exists()


def prepared_panel(tmp_path,monkeypatch):
    from test_frozen_mode_campaign import prepare_fixture
    schedule,qual,source=prepare_fixture(tmp_path,monkeypatch)
    panel=tmp_path/'prepared-panel'
    q.campaign.prepare(panel,schedule,qual,qual,source/'parity.json',source/'profile.json','a-cp12-vs-cp8','linux',3)
    monkeypatch.setattr(q.shutil,'disk_usage',lambda _:SimpleNamespace(free=100*q.GIB))
    return panel


def test_prepare_freezes_copy_and_standalone_imports(tmp_path,monkeypatch):
    panel=prepared_panel(tmp_path,monkeypatch);root=tmp_path/'prepared-queue'
    result=q.prepare(root,[panel],q.GIB,Path('/opt/program/host.lock'))
    assert result==dict(state='prepared',panels=1,launched=False)
    assert q.validate(root)['panels'][0]['manifest_sha256']==q.digest(panel/'manifest.json')
    import sys
    completed=subprocess.run([sys.executable,str(root/'host_controller.py'),'validate','--root',str(root)],capture_output=True,text=True)
    assert completed.returncode==0,completed.stderr
    assert not (root/'started.json').exists()
    (root/'panels/a-cp12-vs-cp8/assets/cards.json').write_text('changed')
    with pytest.raises(ValueError,match='changed frozen file'):q.validate(root)


def test_duplicate_preparation_rejected_before_copy(tmp_path,monkeypatch):
    panel=prepared_panel(tmp_path,monkeypatch);root=tmp_path/'prepared-queue'
    with pytest.raises(ValueError,match='duplicate panel'):q.prepare(root,[panel,panel],q.GIB,Path('/opt/program/host.lock'))
    assert not root.exists()


def test_preparation_disk_guard_before_copy(tmp_path,monkeypatch):
    panel=prepared_panel(tmp_path,monkeypatch);root=tmp_path/'prepared-queue'
    monkeypatch.setattr(q.shutil,'disk_usage',lambda _:SimpleNamespace(free=q.GIB))
    with pytest.raises(ValueError,match='disk headroom'):q.prepare(root,[panel],q.GIB,Path('/opt/program/host.lock'))
    assert not root.exists()


@pytest.mark.parametrize('path,platform,expected',[
    ('/opt/program/host.lock','linux','/opt/program/host.lock'),
    (r'\opt\program\host.lock','linux','/opt/program/host.lock'),
    (r'D:\program\host.lock','windows','D:/program/host.lock'),
])
def test_destination_host_paths_do_not_depend_on_preparation_os(path,platform,expected):
    assert q.normalized_lock(path,platform)==expected


@pytest.mark.parametrize('path,platform', [('relative.lock','linux'),('relative.lock','windows'),
    ('D:/program/host.lock','linux'),('/opt/program/host.lock','windows')])
def test_invalid_destination_host_paths_rejected(path,platform):
    with pytest.raises(ValueError):q.normalized_lock(path,platform)


def test_archive_preserves_linux_executable_and_empty_directories(tmp_path,monkeypatch):
    panel=prepared_panel(tmp_path,monkeypatch);root=tmp_path/'prepared-queue'
    q.prepare(root,[panel],q.GIB,Path('/opt/program/host.lock'))
    dest=tmp_path/'deployment.tar.gz';result=q.archive(root,dest)
    assert result['native_executables']==1 and not result['launched']
    with q.tarfile.open(dest) as bundle:
        binary=q.read(panel/'protocol.json')['binary']
        assert bundle.getmember('panels/a-cp12-vs-cp8/'+binary).mode & 0o111
        assert bundle.getmember('panels/a-cp12-vs-cp8/results').isdir()
    with pytest.raises(ValueError,match='new archive'):q.archive(root,dest)
    (root/'started.json').write_text('{}')
    with pytest.raises(ValueError,match='already attempted'):q.archive(root,tmp_path/'second.tar.gz')
