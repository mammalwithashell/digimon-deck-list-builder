import copy
import pytest
from tools.cloud_search.profile_frozen_mode_preflight import sampled_peak,timing_summary,profile


def sample(root):
    return dict(working_set_unit='bytes',peak_working_set_unit='KiB',processes=[
        dict(Name='frozen_checkpoint_modes.exe',CommandLine=str(root/'runtime/frozen_checkpoint_modes.exe')+' requests/x.json results/x.json',
             ProcessId=5,WorkingSetSize=1000,PeakWorkingSetSize=2),
        dict(Name='python.exe',ProcessId=1,PeakWorkingSetSize=1000000)])


def test_kib_converted_and_wrapper_ignored(tmp_path):
    s=sample(tmp_path)
    assert sampled_peak(tmp_path,dict(binary='runtime/frozen_checkpoint_modes.exe'),[s,s])==(2048,2,1)


@pytest.mark.parametrize('mutation',[
    lambda s:s.update(peak_working_set_unit='bytes'),
    lambda s:s['processes'][0].update(CommandLine='C:/other/frozen_checkpoint_modes.exe a b'),
    lambda s:s['processes'][0].update(PeakWorkingSetSize=0),
    lambda s:s['processes'][0].update(WorkingSetSize=10000),
    lambda s:s.update(processes=[]),
])
def test_bad_or_foreign_memory_is_not_a_profile(tmp_path,mutation):
    s=sample(tmp_path);mutation(s)
    with pytest.raises(ValueError):sampled_peak(tmp_path,dict(binary='runtime/frozen_checkpoint_modes.exe'),[s])


def test_timings_use_both_games_and_search_denominators():
    result=timing_summary([dict(games=[dict(seconds=3,search_diagnostics=[dict(diagnostics={'seconds':1})]),
                                     dict(seconds=7,search_diagnostics=[dict(diagnostics={'seconds':2}),dict(diagnostics={'seconds':3})])])])
    assert result['games']==2 and result['game_seconds_mean']==5 and result['request_game_seconds_max']==10
    assert result['searched_decisions']==3 and result['search_seconds_mean']==2


def test_incomplete_proof_never_writes_profile(tmp_path):
    from tools.cloud_search.frozen_mode_campaign import write
    write(tmp_path/'protocol.json',dict(platform='windows',deadline_seconds=7200))
    write(tmp_path/'completion-verified.json',dict(requests=1,games=2,scored=False))
    out=tmp_path/'profile.json'
    with pytest.raises(ValueError,match='complete qualification'):profile(tmp_path,[],out)
    assert not out.exists()


def test_complete_profile_binds_receipts_outputs_and_memory(tmp_path):
    from tools.cloud_search.frozen_mode_campaign import write,digest,read
    (tmp_path/'results').mkdir()
    write(tmp_path/'protocol.json',dict(platform='windows',deadline_seconds=7200,binary='runtime/frozen_checkpoint_modes.exe'))
    write(tmp_path/'manifest.json',dict(files={}))
    receipts={}
    for i in range(134):
        name=f'job{i}';out=tmp_path/f'results/{name}.json'
        write(out,dict(games=[dict(seconds=2,search_diagnostics=[dict(diagnostics={'seconds':1})]) for _ in range(2)]))
        receipt=tmp_path/f'results/{name}.done.json';write(receipt,dict(output_sha256=digest(out)))
        receipts[name]=digest(receipt)
    write(tmp_path/'completion-verified.json',dict(requests=134,games=268,scored=False,heldout_fallback_worlds=1,
        manifest_sha256=digest(tmp_path/'manifest.json'),receipt_hashes=receipts))
    evidence=tmp_path/'monitor.json';write(evidence,sample(tmp_path))
    output=tmp_path/'profile.json';result=profile(tmp_path,[evidence],output)
    assert result['verified'] and result['games']==268 and result['observed_peak_worker_rss_bytes']==2048
    assert result['memory_evidence_sha256']=={'monitor.json':digest(evidence)}
    with pytest.raises(ValueError,match='preserve existing'):profile(tmp_path,[evidence],output)
    write(tmp_path/'results/job0.json',{})
    with pytest.raises(ValueError,match='output/receipt changed'):profile(tmp_path,[evidence],tmp_path/'second.json')
