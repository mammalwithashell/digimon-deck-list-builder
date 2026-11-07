"""Complete-only Windows resource profile from verified receipts and explicit-unit samples."""
import argparse
import math
from pathlib import Path
import time
from .frozen_mode_campaign import read,write,digest,require


def sampled_peak(root,protocol,snapshots):
    expected=(root.resolve()/protocol['binary']).as_posix().lower()
    peaks=[];pids=set()
    for snapshot in snapshots:
        require(snapshot.get('working_set_unit')=='bytes' and snapshot.get('peak_working_set_unit')=='KiB',
                'explicit Windows memory units required')
        for process in snapshot.get('processes',[]):
            if process.get('Name','').lower()!='frozen_checkpoint_modes.exe':continue
            command=process.get('CommandLine','').replace('\\','/').lower()
            # The native binary is the first argument; never accept another root's worker.
            require(command.startswith(expected+' ') or command.startswith('"'+expected+'" '), 'foreign native worker sample')
            peak=int(process['PeakWorkingSetSize'])*1024
            current=int(process['WorkingSetSize'])
            require(peak>0 and current>0 and peak>=current,'invalid worker memory sample')
            peaks.append(peak);pids.add(process['ProcessId'])
    require(peaks,'no exact native worker memory samples')
    return max(peaks),len(peaks),len(pids)


def timing_summary(outputs):
    games=[];requests=[];search=[]
    for output in outputs:
        row=[]
        for game in output['games']:
            value=game['seconds'];require(math.isfinite(value) and value>=0,'invalid game timing')
            row.append(value)
            for decision in game['search_diagnostics']:
                value=decision['diagnostics']['seconds']
                require(math.isfinite(value) and value>=0,'invalid search timing');search.append(value)
        require(len(row)==2,'two games per request');games.extend(row);requests.append(sum(row))
    require(games and search,'missing native timing evidence')
    return dict(games=len(games),game_seconds_mean=sum(games)/len(games),game_seconds_max=max(games),
                request_game_seconds_max=max(requests),searched_decisions=len(search),
                search_seconds_mean=sum(search)/len(search),search_seconds_max=max(search))


def profile(root,samples,out):
    require(not out.exists(),'preserve existing runtime profile')
    protocol=read(root/'protocol.json');complete=read(root/'completion-verified.json')
    require(protocol['platform']=='windows' and protocol['deadline_seconds']==7200,'Windows qualification protocol')
    require(complete['requests']==134 and complete['games']==268 and complete['scored'] is False
            and complete['heldout_fallback_worlds']>0,'complete qualification proof required')
    require(complete['manifest_sha256']==digest(root/'manifest.json'),'qualification manifest changed')
    expected=set(complete['receipt_hashes'])
    require(len(expected)==134 and {p.name.removesuffix('.done.json') for p in (root/'results').glob('*.done.json')}==expected
            and not list((root/'results').glob('*.rejected.json')),'complete successful receipts required')
    outputs=[]
    for job,sha in complete['receipt_hashes'].items():
        receipt=root/f'results/{job}.done.json';output=root/f'results/{job}.json'
        require(digest(receipt)==sha and digest(output)==read(receipt)['output_sha256'],'verified output/receipt changed')
        outputs.append(read(output))
    samples=sorted(set(p.resolve() for p in samples))
    peak,count,pids=sampled_peak(root,protocol,[read(p) for p in samples])
    timings=timing_summary(outputs);require(timings['games']==268,'timing denominator')
    result=dict(verified=True,at=time.time(),qualification_manifest_sha256=digest(root/'manifest.json'),
        completion_proof_sha256=digest(root/'completion-verified.json'),observed_completed_requests=134,
        observed_peak_worker_rss_bytes=peak,memory_samples=count,distinct_sampled_worker_pids=pids,
        memory_evidence_sha256={p.name:digest(p) for p in samples},
        memory_scope='Maximum sampled Win32_Process PeakWorkingSetSize * 1024 (KiB to bytes); not an exhaustive worker peak. Keep allocation headroom.',
        decision_cap=2000,request_deadline_seconds=7200,**timings)
    write(out,result);return result


if __name__=='__main__':
    import json
    p=argparse.ArgumentParser();p.add_argument('--root',type=Path,required=True);p.add_argument('--out',type=Path,required=True)
    p.add_argument('--samples',type=Path,nargs='+',required=True);a=p.parse_args()
    print(json.dumps(profile(a.root,a.samples,a.out),indent=2))
