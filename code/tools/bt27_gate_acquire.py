"""Acquire review-only full-pool sources with explicit resumable response pins.

Never publishes, allocates IDs, or changes production artifacts. Cached bytes
retain their original acquisition timestamp; a failed request is not relabeled.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime, timezone
import hashlib
import io
import json
from pathlib import Path
import threading
import urllib.request

from tools.card_data.cache import ContentAddressedResponseCache, fetch_with_retry
from tools.card_data.live_reconcile import _secondary_snapshot, build_reconciliation, write_reconciliation_bundle
from tools.card_data.serialization import canonical_json_text
from tools.card_data.sources.official import OFFICIAL_CATALOG_URL, OfficialSourceAdapter, parse_catalog
from tools.card_data.sources.digimoncard_io import DigimonCardIOAdapter


def main():
    ap=argparse.ArgumentParser();ap.add_argument('--repo-root',type=Path,default=Path.cwd());ap.add_argument('--output-dir',type=Path,required=True);args=ap.parse_args()
    out=args.output_dir.resolve();out.mkdir(parents=True,exist_ok=True)
    cache=ContentAddressedResponseCache(out/'responses');index_path=out/'response-index.json';lock=threading.Lock()
    index=json.loads(index_path.read_text()) if index_path.exists() else {}
    def remember(url,body):
        digest,_=cache.put(body)
        with lock:
            index[url]={'sha256':digest,'acquired_at':datetime.now(timezone.utc).isoformat()}
            index_path.write_text(json.dumps(index,indent=2)+'\n')
        print('acquired',url,digest,flush=True)
    def fetch(url):
        if url in index:return cache.get(index[url]['sha256'])
        result=fetch_with_retry(url,cache=cache,timeout=30);remember(url,result.body);return result.body
    errors={};official=None;secondary=None
    try:
        catalog=fetch(OFFICIAL_CATALOG_URL);entries=parse_catalog(catalog,source_url=OFFICIAL_CATALOG_URL)
        with ThreadPoolExecutor(max_workers=3) as pool:
            futures={pool.submit(fetch,e.source_url):e for e in entries}
            for future in as_completed(futures):
                e=futures[future]
                try:future.result()
                except Exception as exc:errors[e.source_url]=str(exc);print('failed',e.category_id,str(exc),flush=True)
        if not errors:
            stamp=min(v['acquired_at'] for k,v in index.items() if 'world.digimoncard.com' in k)
            official=OfficialSourceAdapter(fetcher=fetch).acquire_catalog(first_acquired_at=stamp)
            (out/'official-snapshot.json').write_text(canonical_json_text(official))
            print('official',len(official.observations),len(official.manifest.observation_refs),flush=True)
    except Exception as exc:errors['official']=repr(exc);print('official failed',repr(exc),flush=True)
    class Response(io.BytesIO):
        status=200
    def opener(request,**kwargs):
        url=request.full_url
        if url in index:return Response(cache.get(index[url]['sha256']))
        with urllib.request.urlopen(request,**kwargs) as response:body=response.read();status=response.status
        if status!=200:raise RuntimeError(f'unexpected HTTP {status}')
        remember(url,body);return Response(body)
    try:
        adapter=DigimonCardIOAdapter(cache=cache,opener=opener)
        catalog=adapter.acquire_catalog()
        stamp=min(v['acquired_at'] for k,v in index.items() if 'digimoncard.io' in k)
        secondary=_secondary_snapshot(catalog.acquisitions,acquired_at=stamp,discovery_url=catalog.discovery_url)
        (out/'secondary-snapshot.json').write_text(canonical_json_text(secondary))
        print('secondary',len(secondary.observations),flush=True)
    except Exception as exc:errors['secondary']=repr(exc);print('secondary failed',repr(exc),flush=True)
    (out/'failures.json').write_text(json.dumps(errors,indent=2)+'\n')
    if official is not None and secondary is not None:
        payload,candidates=build_reconciliation(args.repo_root.resolve(),official=official,secondary=secondary)
        write_reconciliation_bundle(out,payload,candidates)
        print('review tree ready',flush=True);return 0
    return 2

if __name__=='__main__':raise SystemExit(main())
