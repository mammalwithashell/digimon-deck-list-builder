"""Complete-only paired continuation report. Never starts or repeats native games."""
import argparse
from collections import Counter, defaultdict
import html
import importlib.util
import json
from pathlib import Path
import random
import statistics
import time

from tools.cloud_search.mulligan_continuation_controller import CHECKPOINTS, digest, read, require, write
from tools.cloud_search.search_opportunity_report import summarize as opportunity_summary


def interval(nodes, values, draws=4000):
    """Resample whole original source blocks within namespace, not game repeats."""
    require(len(nodes) == len(values) > 0, "empty or misaligned estimate")
    groups = defaultdict(lambda: defaultdict(list))
    for node, value in zip(nodes, values):
        groups[node['namespace']][node['block']].append(value)
    rng = random.Random(10426103000)
    boot = []
    for _ in range(draws):
        sample = []
        for namespace in sorted(groups):
            blocks = list(groups[namespace].values())
            for _ in blocks:
                sample.extend(rng.choice(blocks))
        boot.append(statistics.mean(sample))
    boot.sort()
    return dict(mean=statistics.mean(values), ci95=[boot[int(.025*draws)], boot[min(draws-1, int(.975*draws))]],
                roots=len(nodes), source_blocks=sum(len(b) for b in groups.values()), bootstrap_draws=draws,
                method='Original source-block percentile bootstrap within namespace; two outcomes averaged per root')


def paired_summary(nodes, rows, draws=4000):
    """Rows are already verified games for exactly one checkpoint."""
    table = {}
    for row in rows:
        key = (row['root'], row['repeat'], row['branch'])
        require(key not in table, 'duplicate game')
        require(row['score'] in (0, .5, 1), 'invalid score')
        table[key] = row
    expected = {(n['id'], r, b) for n in nodes for r in (0, 1) for b in (0, 1)}
    require(set(table) == expected, 'incomplete root/repeat/branch coverage')
    differences, branch_values = [], {0: [], 1: []}
    for node in nodes:
        for repeat in (0, 1):
            a, b = (table[(node['id'], repeat, branch)] for branch in (0, 1))
            require(a['host'] == b['host'], 'paired branches split across hosts')
            require(a['outcome_seed'] == b['outcome_seed'] and a['search_seed'] == b['search_seed'], 'unpaired RNG')
        for branch in (0, 1):
            branch_values[branch].append(statistics.mean(table[(node['id'], r, branch)]['score'] for r in (0, 1)))
        differences.append(branch_values[0][-1] - branch_values[1][-1])
    branches = {}
    for branch, label in ((0, 'keep'), (1, 'redraw')):
        scores = [r['score'] for r in rows if r['branch'] == branch]
        branches[label] = dict(games=len(scores), wins=scores.count(1), losses=scores.count(0), draws=scores.count(.5),
                               score=interval(nodes, branch_values[branch], draws))
    return dict(games=len(rows), unique_roots=len(nodes), paired_outcomes=2*len(nodes), branches=branches,
                keep_minus_redraw=interval(nodes, differences, draws))


def analyze(nodes, rows, draws=4000):
    require(len(nodes) == len({n['id'] for n in nodes}) == 52, '52 unique preregistered roots required')
    require(len(rows) == 832 and {r['checkpoint'] for r in rows} == set(CHECKPOINTS), '832 games/all checkpoints required')
    output = {}
    for cp in CHECKPOINTS:
        games = [r for r in rows if r['checkpoint'] == cp]
        result = paired_summary(nodes, games, draws)
        for field, label in (('archetype', 'archetypes'), ('first', 'opener'), ('mover', 'physical_seat'), ('namespace', 'source_namespace')):
            result[label] = {}
            for value in sorted({n[field] for n in nodes}):
                subset = [n for n in nodes if n[field] == value]
                ids = {n['id'] for n in subset}
                result[label][str(value)] = paired_summary(subset, [g for g in games if g['root'] in ids], draws)
        output[str(cp)] = result
    return output


class Behavior:
    """Stream compact per-game summaries, without retaining full native telemetry."""
    def __init__(self):
        self.groups = defaultdict(lambda: dict(games=0, events=Counter(), counts=Counter(), means={}, timings=[]))

    def add(self, cp, host, branch, game):
        for role, seat in (('candidate', game['candidate_seat']), ('opponent', 1-game['candidate_seat'])):
            key = (cp, host, branch, role)
            group = self.groups[key]
            group['games'] += 1
            searches = [s['opportunity'] for s in game['search_diagnostics'] if s['actor'] == seat]
            summary = opportunity_summary(searches)
            group['counts'].update(summary['counts'])
            for metric, estimate in summary['means'].items():
                total, count = group['means'].get(metric, (0., 0))
                n = estimate['measured_roots']
                group['means'][metric] = (total + estimate['value']*n, count+n)
            for step in game['card_metrics']['steps']:
                if step['mover'] == seat+1 and len(step['legal_actions']) > 1 and not step.get('forced_intervention', False):
                    group['timings'].append(step['selection_seconds'])
                for event in step['events']:
                    if event.get('player') != seat:
                        continue
                    kind = event['type']
                    if kind in ('Digivolve', 'EvalDigivolve'):
                        # Actual native event metadata, not inference from selected plans.
                        route = 'dna' if event.get('was_dna', False) else ('raising' if event['field_index'] == 14 else 'battle')
                        group['events']['digivolve_' + route] += 1
                    elif kind == 'BreedingMove':
                        group['events']['promotion'] += 1
                    elif kind in ('Play', 'Hatch'):
                        group['events'][kind.lower()] += 1

    def report(self):
        result = []
        for (cp, host, branch, role), group in sorted(self.groups.items()):
            times = sorted(group['timings'])
            result.append(dict(checkpoint=cp, host=host, branch=branch, role=role, games=group['games'],
                resolved_routes=dict(group['events']), opportunity_counts=dict(group['counts']),
                opportunity_means={k:dict(value=total/count, measured_roots=count) for k,(total,count) in group['means'].items()},
                timing=dict(nonforced_actions=len(times), total_seconds=sum(times),
                    mean_seconds=statistics.mean(times) if times else None,
                    median_seconds=statistics.median(times) if times else None,
                    p95_seconds=times[min(len(times)-1,int(.95*len(times)))] if times else None)))
        return dict(groups=result, note='Roles describe the same games. Forced intervention excluded from decision timing. Routes describe events after the saved root. Direct searched opportunities exclude forced roots; effect-sequence opportunities unmeasured. Decisions are correlated; evaluation uses argmax tau=0/no noise, not training exploration.')


def load_verified(windows, linux):
    """Use each frozen controller, binding the two complete shards before reporting."""
    rows, evidence, behavior = [], {}, Behavior()
    reference = None
    all_jobs = set()
    for host, root in (('windows', windows), ('linux', linux)):
        protocol = read(root/'protocol.json')
        require(protocol['stage'] == 'scored_continuations' and protocol['platform'] == host, 'wrong stage/host')
        require(protocol['repeat'] == (0 if host == 'windows' else 1), 'wrong repeat allocation')
        manifest = read(root/'manifest.json')['files']
        require(digest(root/'controller.py') == manifest['controller.py'], 'frozen controller hash')
        spec = importlib.util.spec_from_file_location('verified_continuation_'+host, root/'controller.py')
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        proof = module.verify(root)
        require(proof['games'] == 416 and proof['scored'] is True, 'incomplete shard')
        nodes = read(root/'inputs/scored-roots.json')
        shared = {f:digest(root/'inputs'/f) for f in ('scored-roots.json','scored-schedule.json','preregistered-protocol.json','qualification-parity.json')}
        require(reference is None or reference == shared, 'shards have different preregistration/qualification')
        reference = shared
        evidence[host] = dict(root=str(root.resolve()), manifest_sha256=digest(root/'manifest.json'),
                             completion_sha256=digest(root/'completion-verified.json'), receipts=proof['receipts'])
        for job in protocol['jobs']:
            require(job not in all_jobs, 'duplicate job across shards')
            all_jobs.add(job)
            q = read(root/f'requests/{job}.json')
            g = read(root/f'results/{job}.json')['game']
            rows.append(dict(job=job, checkpoint=q['checkpoint'], root=q['root'], repeat=q['repeat'],
                branch=q['forced_mulligan_action'], outcome_seed=q['outcome_seed'], search_seed=q['follow_on_search_seed'],
                score=g['score'], seconds=g['seconds'], host=host))
            behavior.add(q['checkpoint'], host, q['forced_mulligan_action'], g)
    require(all_jobs == {j['id'] for j in read(windows/'inputs/scored-schedule.json')} and len(all_jobs) == 832, 'schedule union')
    return nodes, rows, evidence, behavior.report()


def report(windows, linux, out):
    require(not out.exists(), 'preserve existing report')
    nodes, rows, evidence, behavior = load_verified(windows, linux)
    result = dict(schema='paired-mulligan-continuations-report-v1', games=832, unique_roots=52,
        per_checkpoint=analyze(nodes, rows), information_regime='known-list', fixed_opponent='CP8',
        caveats=['Outcomes conditional on each frozen candidate follow-on pilot and fixed CP8 opponent.',
            'Two independent outcome seeds per root, paired keep/redraw branches on the same host; subsequent RNG consumption can diverge.',
            '52 roots/104 paired outcomes per checkpoint; four roots per archetype give sparse uncertainty.',
            'Source blocks and shared model seeds are correlated; do not pool checkpoints or role reports.',
            'Exploratory comparisons, not universally optimal mulligans, human strength or promotion evidence.'])
    out.mkdir(parents=True)
    write(out/'summary.json', result)
    write(out/'behavior.json', behavior)
    write(out/'paired-game-scores.json', rows)
    table = []
    for cp, summary in result['per_checkpoint'].items():
        diff = summary['keep_minus_redraw']
        a,b = (summary['branches'][k] for k in ('keep','redraw'))
        table.append(f'<tr><td>CP{cp}</td><td>{a["wins"]}/{a["losses"]}/{a["draws"]}</td><td>{b["wins"]}/{b["losses"]}/{b["draws"]}</td><td>{100*diff["mean"]:.2f} pp</td><td>{100*diff["ci95"][0]:.2f} to {100*diff["ci95"][1]:.2f} pp</td></tr>')
    page = '<!doctype html><meta charset="utf-8"><title>Paired mulligan outcomes</title><style>body{font:17px system-ui;max-width:1100px;margin:40px auto;padding:20px;background:#101b2a;color:#eee}td,th{padding:12px;text-align:left}a{color:#85caff}</style><h1>Keep versus redraw</h1><p>832 completed games. Each checkpoint: 52 roots, 104 paired outcomes; 104 games per branch.</p><table><tr><th>Candidate</th><th>Keep W/L/D</th><th>Redraw W/L/D</th><th>Keep minus redraw</th><th>Source-block 95% interval</th></tr>'+''.join(table)+'</table><ul>'+''.join('<li>'+html.escape(c)+'</li>' for c in result['caveats'])+'</ul><p><a href="summary.json">Seat, opener, archetype and namespace results</a> · <a href="behavior.json">Host timing, routes and search opportunities</a> · <a href="paired-game-scores.json">Paired game scores</a></p>'
    (out/'index.html').write_text(page, encoding='utf-8')
    write(out/'completion-verified.json', dict(at=time.time(), inputs=evidence,
        source_files={str(Path(__file__).resolve()):digest(Path(__file__)),
                      'search_opportunity_report.py':digest(Path(__file__).with_name('search_opportunity_report.py'))},
        outputs={p.name:digest(p) for p in out.iterdir() if p.is_file()}))
    return dict(games=832, roots=52, out=str(out))


if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    for field in ('windows','linux','out'):
        parser.add_argument('--'+field,type=Path,required=True)
    print(json.dumps(report(**vars(parser.parse_args()))))
