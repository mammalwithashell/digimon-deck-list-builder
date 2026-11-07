"""Global, disjoint host assignment checks; never authorizes model execution."""
import argparse
from collections import Counter
from pathlib import Path
from .frozen_mode_campaign import read, digest, require


def validate_assignment(panels, assignment):
    expected = {p['id']: p for p in panels}
    rows = assignment['panels']
    require(len(expected) == len(panels), 'duplicate scheduled panel')
    require(Counter(r['id'] for r in rows) == Counter(expected.keys()),
            'global coverage must contain every panel exactly once')
    by_id = {r['id']: r for r in rows}
    hosts = assignment['hosts']
    for row in rows:
        require(row['host'] in hosts, 'unknown host')
        require(row['games'] == expected[row['id']]['games'], 'game budget changed')
        require(row['stage'] == expected[row['id']]['stage'], 'stage changed')
        other = expected[row['id']].get('paired_control')
        if other:
            require(row['host'] == by_id[other]['host'], 'matched controls split hosts')
    for host, limits in hosts.items():
        n = sum(r['host'] == host for r in rows)
        required = (n * limits['artifact_bytes_per_panel'] + limits['staging_bytes']
                    + limits['report_bytes'] + limits['disk_reserve_bytes'])
        require(all(limits[k] > 0 for k in ('artifact_bytes_per_panel', 'staging_bytes',
                    'report_bytes', 'disk_reserve_bytes')), 'positive artifact estimates required')
        require(required <= limits['observed_free_bytes'], 'retained artifact budget exceeds disk')
    return dict(panels=len(rows), games=sum(r['games'] for r in rows),
                disjoint=True, matched_controls_same_host=True, launch_allowed=False)


def validate(root, schedule):
    manifest = read(root/'manifest.json')
    for name, sha in manifest['files'].items():
        path = (root/name).resolve()
        require(path.is_relative_to(root.resolve()) and digest(path) == sha, 'allocation file changed')
    assignment = read(root/'assignment.json')
    require(assignment['schedule_manifest_sha256'] == digest(schedule/'manifest.json'),
            'schedule identity changed')
    for name, sha in read(schedule/'manifest.json')['files'].items():
        require(digest(schedule/name) == sha, 'schedule file changed')
    return assignment, validate_assignment(read(schedule/'panels.json'), assignment)


def validate_queue(root, schedule, host, queue_root):
    assignment, proof = validate(root, schedule)
    require(host in assignment['hosts'], 'unknown host')
    protocol = read(queue_root/'protocol.json')
    require(protocol['schedule_manifest_sha256'] == assignment['schedule_manifest_sha256'],
            'queue schedule mismatch')
    require(protocol['platform'] == assignment['hosts'][host]['platform'], 'queue platform mismatch')
    require(protocol['shared_host_lock'] == assignment['hosts'][host]['shared_lock'], 'host lock mismatch')
    ids = [p['id'] for p in protocol['panels']]
    require(len(ids) == len(set(ids)) and ids, 'duplicate/empty queue')
    allowed = {p['id'] for p in assignment['panels'] if p['host'] == host}
    require(set(ids) <= allowed, 'queue crosses global host assignment')
    require(protocol['artifact_bytes_per_panel'] >= assignment['hosts'][host]['artifact_bytes_per_panel'],
            'queue artifact budget reduced')
    return dict(**proof, queue_panels=ids, host=host, queue_manifest_sha256=digest(queue_root/'manifest.json'))


if __name__ == '__main__':
    import json
    p = argparse.ArgumentParser()
    p.add_argument('--root', type=Path, required=True)
    p.add_argument('--schedule', type=Path, required=True)
    p.add_argument('--host')
    p.add_argument('--queue', type=Path)
    args = p.parse_args()
    require(bool(args.host) == bool(args.queue), 'host and queue must be supplied together')
    print(json.dumps(validate_queue(args.root, args.schedule, args.host, args.queue)
          if args.queue else validate(args.root, args.schedule)[1], indent=2))
