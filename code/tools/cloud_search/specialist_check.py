"""Specialization check: does specialist A pilot A better than the base model?

For every archetype A, a fixed schedule of single games: the candidate pilots
one of A's lists, the frozen base model pilots a list of another archetype (no
mirrors). Each pair of games shares decks and seed, with the candidate opening
once and going second once. The same schedule is played twice:

  baseline  candidate = base model (the league's starting point);
  genN      candidate = specialist A from league generation N.

Same seeds and decks, so the per-game difference genN - baseline is a paired
estimate of what specialization added (both seats search 320/4, no root noise).
The schedule depends only on the deck pool and seed, so every generation is
comparable with one baseline. Never trains; local native harness only.

usage:
  specialist_check.py prepare --root R --engine EXE --inputs DIR --label baseline|genN [--models DIR]
  specialist_check.py run --root R [--workers 6]
  specialist_check.py compare --baseline R0 --candidate R1
"""
from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import json
from pathlib import Path
import random
import shutil

from tools.selfplay_scheduler import atomic_json, sha256

SEARCH = dict(id='search-320-4', kind='search', simulations=320, worlds=4)
DATA_FILES = ('cards.json', 'card_admissibility.json', 'mechanics_readiness.json')


def archetype(name):
    return name.split(':sha256:', 1)[0]


def slug(name):
    return ''.join(c.lower() if c.isalnum() else '-' for c in name).strip('-').replace('--', '-')


def schedule(decks, games_per_archetype=40, seed=7_302_611_080):
    """Per archetype: pairs (candidate first, candidate second) on one even
    seed; A's lists and the other archetypes are cycled evenly."""
    if games_per_archetype % 2 or seed % 2:
        raise ValueError('Use an even game count and an even base seed')
    groups = {}
    for index, deck in enumerate(decks):
        groups.setdefault(archetype(deck['name']), []).append(index)
    names = sorted(groups)
    out = {}
    for a_index, a in enumerate(names):
        rng = random.Random(seed + a_index)
        others = [n for n in names if n != a]
        rng.shuffle(others)
        rows = []
        for pair in range(games_per_archetype // 2):
            mine = groups[a][pair % len(groups[a])]
            other = others[pair % len(others)]
            theirs = groups[other][(pair // len(others)) % len(groups[other])]
            game_seed = seed + 10_000 * a_index + 2 * pair * 10
            for opener, (indices, seat) in enumerate((([mine, theirs], 0), ([theirs, mine], 1))):
                rows.append(dict(id=f'pair-{pair:03d}-{opener}', pair=pair, deck_indices=indices,
                                 candidate_seats=[seat], game_seed=game_seed,
                                 search_seed=game_seed + 1_000_000 + opener,
                                 candidate=SEARCH, opponent=SEARCH, card_metrics=False))
        out[a] = rows
    return out


def prepare(args):
    root = args.root.resolve()
    if root.exists():
        raise FileExistsError(f'Preserving {root}')
    inputs = root / 'inputs'
    inputs.mkdir(parents=True)
    for name in DATA_FILES + ('decks.json', 'base.onnx', 'base.onnx.meta.json'):
        shutil.copy2(args.inputs / name, inputs / name)
    runtime = root / 'runtime'
    runtime.mkdir()
    engine = runtime / args.engine.name
    shutil.copy2(args.engine, engine)
    for folder in (args.engine.parent, args.engine.parent.parent):
        for dll in folder.glob('*.dll'):
            if not (runtime / dll.name).exists():
                shutil.copy2(dll, runtime / dll.name)
    decks = json.loads((inputs / 'decks.json').read_text(encoding='utf-8'))
    plans = schedule(decks, args.games_per_archetype)
    for arch, rows in plans.items():
        folder = root / 'archetypes' / slug(arch)
        folder.mkdir(parents=True)
        if args.label == 'baseline':
            model = inputs / 'base.onnx'
        else:
            model = inputs / f'{slug(arch)}.onnx'
            for suffix in ('.onnx', '.onnx.meta.json'):
                shutil.copy2(args.models / f'{slug(arch)}{suffix}', inputs / f'{slug(arch)}{suffix}')
        atomic_json(folder / 'plan.json', dict(
            schema='specialist-check-v1', label=args.label, archetype=arch, model=str(model),
            opponent_model=str(inputs / 'base.onnx'), cards_json=str(inputs / 'cards.json'),
            engine=str(engine), decks=decks, decision_cap=1200))
        atomic_json(folder / 'schedule.json', rows)
    files = {p.relative_to(root).as_posix(): sha256(p) for p in root.rglob('*') if p.is_file()}
    atomic_json(root / 'manifest.json', dict(label=args.label, games_per_archetype=args.games_per_archetype,
                                             archetypes=sorted(plans), files=files))
    print(json.dumps(dict(status='prepared', label=args.label, archetypes=len(plans),
                          games=sum(len(r) for r in plans.values()))), flush=True)


def check_record(request, record):
    games = record['games']
    if len(games) != 1 or games[0]['candidate_seat'] != request['candidate_seats'][0]:
        raise ValueError('Expected one game in the requested candidate seat')
    game = games[0]
    if game['truncated'] or any(x in game['terminal_reason'] for x in ('Crash', 'StepLimit', 'None')):
        raise ValueError('Incomplete game; never scored')
    if game['first_player'] != request['game_seed'] % 2:
        raise ValueError('Opening order differs')
    return game


def run(args):
    from tools.search_quality_study import Jobs
    root = args.root.resolve()
    manifest = json.loads((root / 'manifest.json').read_text())
    for name, digest in manifest['files'].items():
        if sha256(root / name) != digest:
            raise ValueError(f'Frozen input changed: {name}')
    jobs = []
    for arch in manifest['archetypes']:
        folder = root / 'archetypes' / slug(arch)
        plan = json.loads((folder / 'plan.json').read_text())
        runner = Jobs(plan, folder, 1, 3600)
        for row in json.loads((folder / 'schedule.json').read_text()):
            request = {k: v for k, v in row.items() if k not in ('id', 'pair')}
            jobs.append((runner, row['id'], request))
    random.Random(918273).shuffle(jobs)
    with ThreadPoolExecutor(max_workers=args.workers) as pool:
        futures = [pool.submit(lambda j: check_record(j[2], j[0].one(j[1], 'match', j[2])), job) for job in jobs]
        for count, future in enumerate(as_completed(futures), 1):
            future.result()
            if count % 50 == 0:
                print(f'{count}/{len(jobs)} games', flush=True)
    result = summarize(root)
    print(json.dumps(dict(label=manifest['label'], **result['overall'])), flush=True)


def scores(root):
    out = {}
    manifest = json.loads((root / 'manifest.json').read_text())
    for arch in manifest['archetypes']:
        folder = root / 'archetypes' / slug(arch)
        for row in json.loads((folder / 'schedule.json').read_text()):
            path = folder / 'results' / f'{row["id"]}.json'
            if path.exists():
                game = check_record(row, json.loads(path.read_text()))
                out[(arch, row['id'])] = game['score']
    return out


def summarize(root):
    rows = scores(root)
    by = {}
    for (arch, _), score in rows.items():
        by.setdefault(arch, []).append(score)
    result = dict(overall=dict(games=len(rows), score=sum(rows.values()) / len(rows) if rows else None),
                  by_archetype={a: dict(games=len(v), score=sum(v) / len(v)) for a, v in sorted(by.items())})
    atomic_json(root / 'summary.json', result)
    return result


def compare(args):
    base, cand = scores(args.baseline.resolve()), scores(args.candidate.resolve())
    common = sorted(set(base) & set(cand))
    diffs = {}
    for key in common:
        diffs.setdefault(key[0], []).append(cand[key] - base[key])
    pairs = [d for v in diffs.values() for d in v]
    rng = random.Random(5)
    boot = sorted(sum(rng.choice(pairs) for _ in pairs) / len(pairs) for _ in range(4000))
    report = dict(games=len(pairs), baseline=sum(base[k] for k in common) / len(common),
                  candidate=sum(cand[k] for k in common) / len(common),
                  difference=sum(pairs) / len(pairs), difference_ci95=[boot[100], boot[3899]],
                  by_archetype={a: dict(games=len(v), baseline=sum(base[(a, i)] for (b, i) in common if b == a) / len(v),
                                        candidate=sum(cand[(a, i)] for (b, i) in common if b == a) / len(v),
                                        difference=sum(v) / len(v)) for a, v in sorted(diffs.items())})
    print(json.dumps(report, indent=1))
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='mode', required=True)
    p = sub.add_parser('prepare')
    p.add_argument('--root', type=Path, required=True)
    p.add_argument('--engine', type=Path, required=True)
    p.add_argument('--inputs', type=Path, required=True)
    p.add_argument('--label', required=True)
    p.add_argument('--models', type=Path)
    p.add_argument('--games-per-archetype', type=int, default=40)
    r = sub.add_parser('run')
    r.add_argument('--root', type=Path, required=True)
    r.add_argument('--workers', type=int, default=6)
    c = sub.add_parser('compare')
    c.add_argument('--baseline', type=Path, required=True)
    c.add_argument('--candidate', type=Path, required=True)
    args = parser.parse_args()
    {'prepare': prepare, 'run': run, 'compare': compare}[args.mode](args)


if __name__ == '__main__':
    main()
