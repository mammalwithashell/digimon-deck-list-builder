"""Specialist matchup matrix: every archetype's champion against every other's.

For each unordered pair of archetypes (A, B), a fixed schedule of game pairs:
A's model pilots one of A's lists, B's model pilots one of B's, same seed,
A opening once and B opening once. Lists cycle across pairs. No mirrors.
Both seats search 320/4 with no root noise.

Two labels share one schedule (it depends only on the deck pool and seed):

  base       the base model pilots both sides (deck strength alone);
  genN       each archetype's league champion after generation N pilots it.

``compare`` pairs the two per game, so genN - base per cell is what the
pilots added on top of the decks. The base matrix only depends on the decks,
so it is run once and reused for every later generation. Never trains; local
native harness only (``specialist_check`` runs the games).

usage:
  specialist_matrix.py prepare --root R --engine EXE --inputs DIR --label base|genN [--models DIR]
  specialist_matrix.py run --root R [--workers 12]
  specialist_matrix.py report --root R [--baseline R0]
"""
from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import itertools
import json
from pathlib import Path
import random
import shutil

from tools.cloud_search.specialist_check import DATA_FILES, SEARCH, archetype, check_record, slug
from tools.selfplay_scheduler import atomic_json, sha256


def groups_of(decks):
    groups = {}
    for index, deck in enumerate(decks):
        groups.setdefault(archetype(deck['name']), []).append(index)
    return groups


def schedule(decks, pairs_per_cell=4, seed=8_402_611_100):
    """{(A, B): rows} for A < B; rows score from A's side."""
    if seed % 2:
        raise ValueError('Use an even base seed (the opener follows seed parity)')
    groups = groups_of(decks)
    out = {}
    for cell, (a, b) in enumerate(itertools.combinations(sorted(groups), 2)):
        rows = []
        for pair in range(pairs_per_cell):
            mine = groups[a][pair % len(groups[a])]
            theirs = groups[b][(pair + cell) % len(groups[b])]
            game_seed = seed + 1_000 * cell + 20 * pair
            for opener, (indices, seat) in enumerate((([mine, theirs], 0), ([theirs, mine], 1))):
                rows.append(dict(id=f'pair-{pair:03d}-{opener}', pair=pair, deck_indices=indices,
                                 candidate_seats=[seat], game_seed=game_seed,
                                 search_seed=game_seed + 1_000_000 + opener,
                                 candidate=SEARCH, opponent=SEARCH, card_metrics=False))
        out[(a, b)] = rows
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
    models = {}
    for arch in groups_of(decks):
        if args.label == 'base':
            models[arch] = inputs / 'base.onnx'
        else:
            models[arch] = inputs / f'{slug(arch)}.onnx'
            for suffix in ('.onnx', '.onnx.meta.json'):
                shutil.copy2(args.models / f'{slug(arch)}{suffix}', inputs / f'{slug(arch)}{suffix}')
    plans = schedule(decks, args.pairs_per_cell)
    for (a, b), rows in plans.items():
        folder = root / 'cells' / f'{slug(a)}--{slug(b)}'
        folder.mkdir(parents=True)
        atomic_json(folder / 'plan.json', dict(
            schema='specialist-matrix-v1', label=args.label, archetype=a, opponent=b,
            model=str(models[a]), opponent_model=str(models[b]), cards_json=str(inputs / 'cards.json'),
            engine=str(engine), decks=decks, decision_cap=1200))
        atomic_json(folder / 'schedule.json', rows)
    files = {p.relative_to(root).as_posix(): sha256(p) for p in root.rglob('*') if p.is_file()}
    atomic_json(root / 'manifest.json', dict(label=args.label, pairs_per_cell=args.pairs_per_cell,
                                             cells=[[a, b] for a, b in plans], files=files))
    print(json.dumps(dict(status='prepared', label=args.label, cells=len(plans),
                          games=sum(len(r) for r in plans.values()))), flush=True)


def cell_folder(root, a, b):
    return root / 'cells' / f'{slug(a)}--{slug(b)}'


def run(args):
    from tools.search_quality_study import Jobs
    root = args.root.resolve()
    manifest = json.loads((root / 'manifest.json').read_text())
    for name, digest in manifest['files'].items():
        if sha256(root / name) != digest:
            raise ValueError(f'Frozen input changed: {name}')
    jobs = []
    for a, b in manifest['cells']:
        folder = cell_folder(root, a, b)
        runner = Jobs(json.loads((folder / 'plan.json').read_text()), folder, 1, 3600)
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
    print(json.dumps(report(root, None)['overall']), flush=True)


def scores(root):
    """{(A, B, game id): A's score} for completed games."""
    manifest = json.loads((root / 'manifest.json').read_text())
    out = {}
    for a, b in manifest['cells']:
        folder = cell_folder(root, a, b)
        for row in json.loads((folder / 'schedule.json').read_text()):
            path = folder / 'results' / f'{row["id"]}.json'
            if path.exists():
                out[(a, b, row['id'])] = check_record(row, json.loads(path.read_text()))['score']
    return out


def report(root, baseline):
    mine = scores(root)
    base = scores(baseline) if baseline else {}
    names = sorted({k[0] for k in mine} | {k[1] for k in mine})
    cells = {}
    for (a, b, game), s in mine.items():
        for x, y, v, w in ((a, b, s, base.get((a, b, game))), (b, a, 1 - s, None if (a, b, game) not in base else 1 - base[(a, b, game)])):
            c = cells.setdefault((x, y), dict(games=0, score=0.0, paired=0, base=0.0))
            c['games'] += 1
            c['score'] += v
            if w is not None:
                c['paired'] += 1
                c['base'] += w
    matrix = {x: {y: dict(games=c['games'], score=c['score'] / c['games'],
                          **(dict(base=c['base'] / c['paired'], lift=(c['score'] - c['base']) / c['paired'])
                             if c['paired'] == c['games'] else {}))
                  for (x2, y), c in sorted(cells.items()) if x2 == x} for x in names}
    rows = {}
    for x in names:
        vals = matrix[x].values()
        games = sum(v['games'] for v in vals)
        row = dict(games=games, score=sum(v['score'] * v['games'] for v in vals) / games)
        if all('lift' in v for v in vals):
            row['base'] = sum(v['base'] * v['games'] for v in vals) / games
            row['lift'] = row['score'] - row['base']
        rows[x] = row
    result = dict(overall=dict(games=len(mine), cells=len(cells) // 2), archetypes=rows, matrix=matrix)
    atomic_json(root / 'report.json', result)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='mode', required=True)
    p = sub.add_parser('prepare')
    p.add_argument('--root', type=Path, required=True)
    p.add_argument('--engine', type=Path, required=True)
    p.add_argument('--inputs', type=Path, required=True)
    p.add_argument('--label', required=True)
    p.add_argument('--models', type=Path)
    p.add_argument('--pairs-per-cell', type=int, default=4)
    r = sub.add_parser('run')
    r.add_argument('--root', type=Path, required=True)
    r.add_argument('--workers', type=int, default=12)
    q = sub.add_parser('report')
    q.add_argument('--root', type=Path, required=True)
    q.add_argument('--baseline', type=Path)
    args = parser.parse_args()
    if args.mode == 'prepare':
        prepare(args)
    elif args.mode == 'run':
        run(args)
    else:
        print(json.dumps(report(args.root.resolve(), args.baseline and args.baseline.resolve())['archetypes'], indent=1))


if __name__ == '__main__':
    main()
