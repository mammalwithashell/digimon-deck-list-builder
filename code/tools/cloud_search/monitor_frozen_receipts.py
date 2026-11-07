"""Bounded read-only receipt monitoring. Liveness evidence never authorizes retries."""
import argparse
from pathlib import Path
import shlex
import time
from .frozen_mode_campaign import read,write,digest,require


def exact_processes(root,protocol,evidence,now):
    if not evidence or now-evidence.get('at',0)>120 or evidence.get('reachable') is False:
        return None
    base=root.resolve().as_posix().lower()
    expected={base+'/controller.py',base+'/host_controller.py',base+'/'+protocol['binary'].replace('\\','/').lower()}
    matched=[]
    for p in evidence.get('processes',[]):
        command=p.get('CommandLine',p.get('command',[]))
        tokens=shlex.split(command,posix=False) if isinstance(command,str) else command
        normalized=[v.strip('"').replace('\\','/').lower() for v in tokens]
        if any(v in expected for v in normalized):matched.append(p)
    # Container PID/command evidence must be bound to this exact host mount.
    # An unmapped /campaign command is not evidence about this panel.
    for container in evidence.get('containers',[]):
        if not container.get('running') or not container.get('id'):
            continue
        for mount in container.get('mounts',[]):
            if mount.get('type')!='bind' or mount.get('source')!=root.resolve().as_posix():
                continue
            destination=mount.get('destination','').rstrip('/')
            if not destination.startswith('/'):
                continue
            native=destination+'/'+protocol['binary'].replace('\\','/')
            scripts={destination+'/controller.py',destination+'/host_controller.py'}
            for p in container.get('processes',[]):
                tokens=p.get('command',[])
                if not isinstance(tokens,list) or not tokens:
                    continue
                if tokens[0]==native or (len(tokens)>1 and tokens[1] in scripts):
                    matched.append(dict(**p,container_id=container['id'],host_mount=mount['source']))
    return matched


def tail(path,limit=2048):
    with path.open('rb') as f:
        f.seek(0,2);f.seek(max(0,f.tell()-limit));return f.read(limit).decode('utf-8',errors='replace')


def monitor(root,previous=None,process_evidence=None,now=None):
    now=time.time() if now is None else now
    if (root/'result-reported.json').exists():
        return dict(at=now,state='already_reported_do_not_poll',root=str(root),outcome_reported=True)
    protocol=read(root/'protocol.json');identity=digest(root/'manifest.json')
    previous=previous or {}
    require(not previous or previous.get('manifest_sha256')==identity,'comparison belongs to another frozen run')
    require(protocol['requests']>0 and protocol['games']==2*protocol['requests'],'two-game request protocol')
    verified={};errors=[]
    for path in sorted((root/'results').glob('*.done.json')):
        name=path.name.removesuffix('.done.json')
        try:
            receipt=read(path);rh=digest(path)
            require(receipt['counts']['games']==2,'receipt game denominator')
            require(digest(root/f'requests/{name}.json')==receipt['request_sha256'],'request hash changed')
            prior=previous.get('verified_receipts',{}).get(name)
            if prior:
                require(prior['receipt_sha256']==rh,'committed receipt changed')
            else:
                require(digest(root/f'results/{name}.json')==receipt['output_sha256'],'new output hash mismatch')
            verified[name]=dict(receipt_sha256=rh,output_sha256=receipt['output_sha256'],committed_at=receipt['committed_at'])
        except (ValueError,KeyError,OSError) as error:
            errors.append(dict(receipt=path.name,error=str(error)[:600]))
    require(len(verified)<=protocol['requests'],'excess committed coverage')
    present={p.name.removesuffix('.done.json') for p in (root/'results').glob('*.done.json')}
    for name in sorted(set(previous.get('verified_receipts',{}))-present):
        errors.append(dict(receipt=name+'.done.json',error='previously committed receipt disappeared'))
    rejected=[]
    for path in sorted((root/'results').glob('*.rejected.json')):
        value=read(path);name=path.name.removesuffix('.rejected.json')
        request=root/f'requests/{name}.json'
        rejected.append(dict(receipt=path.name,receipt_sha256=digest(path),error=str(value.get('error',''))[:1200],
            request=str(request),request_sha256=digest(request) if request.exists() else None,
            output=str(root/f'results/{name}.json'),preserve='Request, models, seeds, decks, output/trace and logs remain in original root; no retries'))
    started=read(root/'started.json') if (root/'started.json').exists() else None
    state=read(root/'state.json') if (root/'state.json').exists() else {}
    latest=max((v['committed_at'] for v in verified.values()),default=(started or {}).get('at',now))
    gap=max(0,now-latest);processes=exact_processes(root,protocol,process_evidence,now)
    if errors:status='evidence_invalid'
    elif rejected or state.get('state') in ('failed','draining_failed'):status='failed_or_draining'
    elif len(verified)==protocol['requests']:status='all_receipts_present_await_final_validation'
    elif process_evidence and process_evidence.get('reachable') is False:status='unreachable_not_crash_evidence'
    elif started and processes==[]:status='no_matching_process_cause_unverified'
    elif started and gap>=1800:status='no_committed_progress_30m'
    elif started:status='progressing' if processes else 'liveness_unverified'
    else:status='not_started'
    log_tails=[]
    if rejected or (started and gap>=1800):
        for path in sorted((root/'logs').glob('*.err.log'),key=lambda p:p.stat().st_mtime,reverse=True)[:2]:
            log_tails.append(dict(path=str(path),tail=tail(path)))
    return dict(at=now,root=str(root),manifest_sha256=identity,state=status,
        committed_requests=len(verified),committed_games=2*len(verified),expected_requests=protocol['requests'],
        rejected=len(rejected),rejection_evidence=rejected,evidence_errors=errors,verified_receipts=verified,
        new_receipts=len(set(verified)-set(previous.get('verified_receipts',{}))),
        latest_committed_at=latest,no_commit_seconds=gap,request_deadline_seconds=protocol['deadline_seconds'],
        matching_processes=processes,process_evidence_at=(process_evidence or {}).get('at'),bounded_error_logs=log_tails,
        verification_scope='New committed request/output hashes and unchanged receipt hashes only; complete semantics/outcomes require frozen final verifier.',
        action='Observe only; no restart, retry, cancellation, score or promotion')


if __name__=='__main__':
    import json
    p=argparse.ArgumentParser();p.add_argument('--root',type=Path,required=True);p.add_argument('--out',type=Path,required=True)
    p.add_argument('--previous',type=Path);p.add_argument('--process-evidence',type=Path);a=p.parse_args()
    require(not a.out.exists(),'preserve earlier monitor record')
    result=monitor(a.root,read(a.previous) if a.previous else None,read(a.process_evidence) if a.process_evidence else None)
    write(a.out,result)
    print(json.dumps({k:v for k,v in result.items() if k in ['at','state','committed_requests','committed_games','rejected','new_receipts','no_commit_seconds','request_deadline_seconds']}))
