"""A scored queue must reserve the full configured container budget on its host."""
import pytest

from tools.cloud_search import frozen_mode_queue as q


def limits(floor):
    return dict(native_cap=8, cpu_ceiling_percent=90,
                memory_reserve_bytes=8*q.GIB, disk_reserve_bytes=2*q.GIB,
                container_memory_floor_bytes=floor)


def snapshot(memory):
    return dict(logical_cpus=8, cpu_percent=5, native_workers=0,
                build_processes=0, available_memory_bytes=memory,
                disk_free_bytes=20*q.GIB)


def test_full_container_budget_and_host_reserve_are_required():
    policy = limits(20*q.GIB)
    assert q.capacity_reasons(snapshot(28*q.GIB-1), policy, 3, q.GIB, q.GIB) == ['memory headroom']
    assert q.capacity_reasons(snapshot(28*q.GIB), policy, 3, q.GIB, q.GIB) == []
    # If measured demand exceeds the floor, reserve that larger container too.
    assert q.capacity_reasons(snapshot(36*q.GIB-1), policy, 3, 6*q.GIB, q.GIB) == ['memory headroom']
    assert q.capacity_reasons(snapshot(36*q.GIB), policy, 3, 6*q.GIB, q.GIB) == []


@pytest.mark.parametrize('peak,expected', [(q.GIB,20*q.GIB),(6*q.GIB,28*q.GIB)])
def test_docker_command_uses_floor_without_reducing_measured_budget(tmp_path, peak, expected):
    panel=tmp_path/'panel';(panel/'inputs').mkdir(parents=True)
    q.write(panel/'inputs/profile.json', {'observed_peak_worker_rss_bytes':peak})
    q.write(tmp_path/'manifest.json', {'files':{}})
    protocol=dict(platform='linux', image=q.IMAGE, limits=limits(20*q.GIB))
    command,_=q.command(tmp_path,protocol,dict(root='panel',workers=3),0)
    assert int(command[command.index('--memory')+1]) == expected
    assert command[command.index('--cpus')+1] == '3'
    assert command[command.index('--network')+1] == 'none'
    assert '--rm' not in command


@pytest.mark.parametrize('floor', [-1, 1.5, True, '21474836480'])
def test_invalid_memory_floor_is_rejected_before_allocation(floor):
    with pytest.raises(ValueError, match='container memory floor'):
        q.capacity_reasons(snapshot(64*q.GIB),limits(floor),3,q.GIB,q.GIB)
