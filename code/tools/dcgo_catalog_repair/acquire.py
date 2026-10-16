"""Capture official EX13 evidence only; never writes into the source repository/DCGO."""
import argparse
import dataclasses
import hashlib
import json
from pathlib import Path
import sys
from datetime import datetime, timezone

IDS = ['EX13-005','EX13-008','EX13-018','EX13-021','EX13-024','EX13-039','EX13-041','EX13-044','EX13-045']

def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--source-repo', type=Path, required=True, help='Read-only checkout containing tools.card_data official parser')
    ap.add_argument('--out', type=Path, required=True)
    args = ap.parse_args()
    if args.out.exists(): raise ValueError('Evidence output must be new')
    sys.path.insert(0, str(args.source_repo / 'code'))
    from tools.card_data.sources.official import OfficialSourceAdapter, parse_card_list, OFFICIAL_PARSER_VERSION
    adapter = OfficialSourceAdapter()
    corrections_path = args.source_repo / 'data/card_corrections.json'
    corrections = json.loads(corrections_path.read_text(encoding='utf-8'))
    applicable = [c for c in corrections['corrections'] if c['card_id'] in IDS]
    if applicable: raise ValueError('EX13 corrections require explicit reviewed mapping before capture')
    args.out.mkdir(parents=True)
    records = []
    for cid in IDS:
        url = 'https://world.digimoncard.com/cards/?search=true&card_no=' + cid
        raw = adapter.fetch_url(url)
        printings = [p for p in parse_card_list(raw.decode('utf-8'), source_url=url) if p.card_id == cid and p.printing_id == cid]
        if len(printings) != 1: raise ValueError(f'Expected exactly one primary printing for {cid}')
        printing = printings[0]
        picture = adapter.fetch_url(printing.image_url)
        extracted = json.dumps(dict(printing.extracted), ensure_ascii=False, indent=2).encode('utf-8')
        files = {cid+'.html':raw, cid+'.png':picture, cid+'.json':extracted}
        for name, content in files.items(): (args.out/name).write_bytes(content)
        records.append(dict(card_id=cid, source_url=url, image_url=printing.image_url,
                            hashes={name:hashlib.sha256(b).hexdigest() for name,b in files.items()}))
        print(cid, printing.extracted['name'])
    manifest = dict(schema_version=1, acquired_at=datetime.now(timezone.utc).isoformat(),
                    parser_version=OFFICIAL_PARSER_VERSION, records=records,
                    parser_file_sha256=hashlib.sha256((args.source_repo / 'code/tools/card_data/sources/official.py').read_bytes()).hexdigest(),
                    correction_ledger_sha256=hashlib.sha256(corrections_path.read_bytes()).hexdigest(),
                    applicable_corrections=applicable, image_reviewed=False)
    (args.out/'manifest.json').write_text(json.dumps(manifest, indent=2)+'\n', encoding='utf-8')

if __name__ == '__main__': main()
