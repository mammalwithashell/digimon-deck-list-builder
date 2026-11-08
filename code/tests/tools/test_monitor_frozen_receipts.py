from tools.cloud_search.monitor_frozen_receipts import monitor
from tools.cloud_search.frozen_mode_campaign import write,digest
import pytest


def fixture(root):
    for d in ['requests','results','logs']:(root/d).mkdir()
    write(root/'manifest.json',{})
    write(root/'protocol.json',dict(requests=2,games=4,deadline_seconds=7200,binary='runtime/frozen_checkpoint_modes.exe'))
    write(root/'started.json',dict(at=100))
    write(root/'state.json',dict(state='running',at=10000))
    return dict(at=200,processes=[dict(CommandLine=f'"{root}/runtime/frozen_checkpoint_modes.exe" a b',ProcessId=7)])


def commit(root,name='one',at=150):
    req=root/f'requests/{name}.json';out=root/f'results/{name}.json'
    write(req,{});write(out,{'game':'data'})
    write(root/f'results/{name}.done.json',dict(counts={'games':2},request_sha256=digest(req),output_sha256=digest(out),committed_at=at))


def test_heartbeat_timestamp_never_counts_as_progress(tmp_path):
    e=fixture(tmp_path);e['at']=2100
    r=monitor(tmp_path,process_evidence=e,now=2100)
    assert r['state']=='no_committed_progress_30m' and r['no_commit_seconds']==2000
    assert r['request_deadline_seconds']==7200


def test_receipts_progress_and_changed_receipt_is_preserved_failure(tmp_path):
    e=fixture(tmp_path);commit(tmp_path)
    first=monitor(tmp_path,process_evidence=e,now=200)
    assert first['new_receipts']==1 and first['committed_games']==2
    second=monitor(tmp_path,first,e,201);assert second['new_receipts']==0
    commit(tmp_path,at=151)
    changed=monitor(tmp_path,second,e,202)
    assert changed['state']=='evidence_invalid' and changed['evidence_errors']


def test_unreachable_and_absent_do_not_assert_crash(tmp_path):
    fixture(tmp_path)
    assert monitor(tmp_path,process_evidence=dict(at=200,reachable=False),now=200)['state']=='unreachable_not_crash_evidence'
    assert monitor(tmp_path,process_evidence=dict(at=200,processes=[]),now=200)['state']=='no_matching_process_cause_unverified'
    assert monitor(tmp_path,process_evidence=dict(at=0,processes=[]),now=200)['state']=='liveness_unverified'


def test_rejection_preserves_request_and_bounded_error_logs(tmp_path):
    e=fixture(tmp_path);write(tmp_path/'requests/one.json',{'seed':123})
    write(tmp_path/'results/one.rejected.json',dict(error='native failed',request='one'))
    (tmp_path/'logs/one.err.log').write_text('x'*10000)
    r=monitor(tmp_path,process_evidence=e,now=200)
    assert r['state']=='failed_or_draining' and r['rejected']==1
    assert r['rejection_evidence'][0]['request_sha256']==digest(tmp_path/'requests/one.json')
    assert len(r['bounded_error_logs'][0]['tail'])==2048


def test_complete_receipts_are_not_an_outcome_report(tmp_path):
    e=fixture(tmp_path);commit(tmp_path);commit(tmp_path,'two')
    assert monitor(tmp_path,process_evidence=e,now=200)['state']=='all_receipts_present_await_final_validation'


def test_reported_component_is_not_revalidated(tmp_path):
    write(tmp_path/'result-reported.json',{})
    assert monitor(tmp_path,now=200)['state']=='already_reported_do_not_poll'


def test_new_corrupted_output_fails_progress_verification(tmp_path):
    e=fixture(tmp_path);commit(tmp_path);write(tmp_path/'results/one.json',{})
    assert monitor(tmp_path,process_evidence=e,now=200)['state']=='evidence_invalid'


def test_disappeared_committed_receipt_is_not_silent_progress_reset(tmp_path):
    e=fixture(tmp_path);commit(tmp_path)
    first=monitor(tmp_path,process_evidence=e,now=200)
    (tmp_path/'results/one.done.json').unlink()
    assert monitor(tmp_path,first,e,201)['state']=='evidence_invalid'


def test_container_process_requires_exact_live_panel_mount(tmp_path):
    fixture(tmp_path)
    container=dict(id='retained-panel',running=True,mounts=[dict(type='bind',source=tmp_path.resolve().as_posix(),destination='/campaign')],
        processes=[dict(pid=51,command=['/campaign/runtime/frozen_checkpoint_modes.exe','request.json'])])
    evidence=dict(at=200,processes=[],containers=[container])
    result=monitor(tmp_path,process_evidence=evidence,now=200)
    assert result['state']=='progressing' and result['matching_processes'][0]['container_id']=='retained-panel'
    container['mounts'][0]['source']+='/another-panel'
    assert monitor(tmp_path,process_evidence=evidence,now=200)['state']=='no_matching_process_cause_unverified'


def test_stopped_or_unmapped_container_cannot_prove_liveness(tmp_path):
    fixture(tmp_path)
    c=dict(id='old-container',running=False,mounts=[dict(type='bind',source=tmp_path.resolve().as_posix(),destination='/campaign')],
        processes=[dict(command=['/campaign/controller.py'])])
    assert monitor(tmp_path,process_evidence=dict(at=200,containers=[c]),now=200)['state']=='no_matching_process_cause_unverified'
    c['running']=True;c['mounts']=[]
    assert monitor(tmp_path,process_evidence=dict(at=200,containers=[c]),now=200)['state']=='no_matching_process_cause_unverified'
