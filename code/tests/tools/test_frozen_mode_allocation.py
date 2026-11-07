import copy
import pytest
from tools.cloud_search.frozen_mode_allocation import validate_assignment
from tools.cloud_search.frozen_mode_allocation import validate_queue
from tools.cloud_search.frozen_mode_campaign import write, digest


def fixture():
    panels = [dict(id='a', games=520, stage='A', paired_control='b'),
              dict(id='b', games=520, stage='B', paired_control='a'),
              dict(id='d', games=480, stage='D', paired_control=None)]
    limits = dict(artifact_bytes_per_panel=100, staging_bytes=10, report_bytes=10,
                  disk_reserve_bytes=20, observed_free_bytes=1000)
    assignment = dict(hosts={'linux':copy.deepcopy(limits), 'windows':copy.deepcopy(limits)},
                      panels=[dict(id=p['id'],games=p['games'],stage=p['stage'],
                                   host='windows' if p['id']=='d' else 'linux') for p in panels])
    return panels, assignment


def test_disjoint_complete_budget_and_matched_hosts():
    p,a=fixture()
    assert validate_assignment(p,a)==dict(panels=3,games=1520,disjoint=True,
                                        matched_controls_same_host=True,launch_allowed=False)


@pytest.mark.parametrize('change',[
    lambda a:a['panels'].append(copy.deepcopy(a['panels'][0])),
    lambda a:a['panels'].pop(),
    lambda a:a['panels'][0].update(host='windows'),
    lambda a:a['panels'][0].update(games=480),
    lambda a:a['panels'][0].update(stage='D'),
    lambda a:a['panels'][0].update(host='unknown'),
    lambda a:a['hosts']['linux'].update(observed_free_bytes=239),
    lambda a:a['hosts']['linux'].update(artifact_bytes_per_panel=0),
])
def test_rejects_duplicate_missing_moved_or_unfunded_coverage(change):
    p,a=fixture();change(a)
    with pytest.raises(ValueError):validate_assignment(p,a)


@pytest.mark.parametrize('mutation',[None, 'wrong_panel', 'wrong_lock', 'smaller_budget', 'changed_assignment'])
def test_queue_must_match_frozen_global_assignment(tmp_path,mutation):
    panels,a=fixture()
    schedule=tmp_path/'schedule';schedule.mkdir()
    write(schedule/'panels.json',panels)
    write(schedule/'manifest.json',dict(files={'panels.json':digest(schedule/'panels.json')}))
    a['schedule_manifest_sha256']=digest(schedule/'manifest.json')
    for host in a['hosts']:
        a['hosts'][host].update(platform=host,shared_lock='/locks/'+host)
    root=tmp_path/'allocation';root.mkdir()
    write(root/'assignment.json',a)
    write(root/'manifest.json',dict(files={'assignment.json':digest(root/'assignment.json')}))
    q=tmp_path/'queue';q.mkdir()
    protocol=dict(schedule_manifest_sha256=a['schedule_manifest_sha256'],platform='linux',
                  shared_host_lock='/locks/linux',artifact_bytes_per_panel=100,panels=[dict(id='a'),dict(id='b')])
    if mutation=='wrong_panel':protocol['panels']=[dict(id='d')]
    if mutation=='wrong_lock':protocol['shared_host_lock']='/different'
    if mutation=='smaller_budget':protocol['artifact_bytes_per_panel']=99
    if mutation=='changed_assignment':write(root/'assignment.json',{})
    write(q/'protocol.json',protocol);write(q/'manifest.json',{})
    if mutation:
        with pytest.raises(ValueError):validate_queue(root,schedule,'linux',q)
    else:
        assert validate_queue(root,schedule,'linux',q)['queue_panels']==['a','b']
