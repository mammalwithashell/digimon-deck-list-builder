"""Bound replay by complete native chunks without rewriting source shards."""
import hashlib
import json
from collections import Counter
from pathlib import Path

from tools.selfplay_scheduler import sha256


def bounded_window(run_dir: Path, generation: int, window: int, games_per_generation: int, seed: int):
    if min(window, games_per_generation) < 1:
        raise ValueError('Replay window and game cap must be positive')
    selected, records = [], []
    for index in range(max(0, generation - window + 1), generation + 1):
        directories = sorted((run_dir / f'gen{index}' / 'shards').glob('proc*'))
        # Fixed per-source-generation order, independent of wins, game length,
        # current update index or wall clock. Original data remains archived.
        ranked = sorted(directories, key=lambda p: hashlib.sha256(
            f'{seed}:{index}:{p.name}'.encode()).digest())
        games, chosen = 0, []
        for directory in ranked:
            manifest = json.loads((directory / 'manifest.json').read_text())
            count = manifest['games']
            if count <= 0:
                raise ValueError('Invalid source game count')
            if games + count <= games_per_generation:
                chosen.append(directory)
                games += count
        if directories and not chosen:
            raise ValueError('Replay cap is smaller than every complete chunk')
        selected.extend(sorted(chosen))
        records.append(dict(generation=index, games_before_quarantine=games,
            selected=[dict(directory=str(p), manifest_sha256=sha256(p / 'manifest.json'),
                exclusions_sha256=sha256(p / 'training_exclusions.json')
                if (p / 'training_exclusions.json').exists() else None) for p in sorted(chosen)]))
    return selected, dict(schema='bounded-replay-window-v1', cap=games_per_generation,
                         seed=seed, generations=records, sampling_unit='complete_chunk')


def appearance_counts(directories):
    total, seats = Counter(), [Counter(), Counter()]
    for directory in directories:
        manifest = json.loads((directory / 'manifest.json').read_text())
        names = manifest['deck_names']
        for pair in manifest['game_decks']:
            for seat, deck in enumerate(pair):
                name = names[deck].split(':sha256:', 1)[0]
                total[name] += 1
                seats[seat][name] += 1
    return dict(appearances=dict(total), by_seat=[dict(s) for s in seats],
                minimum=min(total.values(), default=0),
                scope='collected games before quarantine; uniform slot sampling')
