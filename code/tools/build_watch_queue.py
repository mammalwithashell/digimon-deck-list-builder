"""Build a UI watch queue from a completed frozen-evaluation panel.

Each game in a frozen-mode panel (PR 760's ``checkpoint_search_eval`` harness)
is stored as a seed, two decklists and the exact executed action list, plus
per-decision search diagnostics. The engine is deterministic from
(decks, seed), so the UI can replay a game by creating a two-human game with
that seed and submitting the recorded actions in order.

Output layout (served by Vite from ``code/frontend/public``)::

    <out>/index.json            queue metadata + one row per game
    <out>/games/<game-id>.json  decks, seed, actions, per-decision search view

Usage::

    python code/tools/build_watch_queue.py --panel <panel-dir> --name cp8-mirror
    python code/tools/build_watch_queue.py --panel <panel-dir> --name cp8-mirror --verify 20

``--panel`` is a directory holding ``plan.json``, ``inputs/jobs.json`` and
``results/<request>.json`` (a live queue panel or an ``archive/panel`` copy).
``--verify N`` replays the first N games through the installed
``digimon_engine`` binding (run it from the repo root so it reads the same
``data/cards.json`` the local server uses) and marks each game
``replayable: false`` if a recorded action is illegal on the current engine
or the game does not end exactly on its last action.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
DEFAULT_OUT_ROOT = REPO / 'code' / 'frontend' / 'public' / 'watch'
TOP_PLANS = 6


def read(path: Path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def mode_label(mode: str) -> str:
    return {'known-list': 'knows list', 'closed-list': 'closed list'}.get(mode, mode)


def search_view(diag: dict) -> dict | None:
    """Compress one decision's diagnostics to what a viewer needs."""
    opp = diag.get('opportunity') or {}
    plans = opp.get('plans') or []
    if not plans:
        return None
    total = sum(p.get('visits') or 0 for p in plans) or 1
    ranked = sorted(plans, key=lambda p: (p.get('visits') or 0, p.get('prior_before_noise') or 0), reverse=True)
    top = ranked[:TOP_PLANS]
    if not any(p.get('selected') for p in top):
        top += [p for p in plans if p.get('selected')][:1]
    return {
        'actor': opp.get('actor'),
        'phase': opp.get('phase'),
        'simulations': (diag.get('diagnostics') or {}).get('simulations'),
        'candidates': len(plans),
        'plans': [
            {
                'label': (p.get('action') or {}).get('label') or str(p.get('plan')),
                'plan': p.get('plan'),
                'prior': round(p.get('prior_before_noise') or 0.0, 4),
                'visits': p.get('visits') or 0,
                'share': round((p.get('visits') or 0) / total, 4),
                'q': None if p.get('q') is None else round(p['q'], 4),
                'selected': bool(p.get('selected')),
            }
            for p in top
        ],
    }


def build(panel_dir: Path, out_dir: Path, title: str) -> list[dict]:
    plan = read(panel_dir / 'plan.json')
    jobs = read(panel_dir / 'inputs' / 'jobs.json')
    (out_dir / 'games').mkdir(parents=True, exist_ok=True)
    rows = []
    for job in jobs:
        result_path = panel_dir / 'results' / f"{job['id']}.json"
        if not result_path.exists():
            continue
        result = read(result_path)
        request = result.get('request') or {}
        cand_mode = (request.get('candidate') or {}).get('information_regime', job.get('candidate_mode', ''))
        opp_mode = (request.get('opponent') or {}).get('information_regime', job.get('opponent_mode', ''))
        decks = [plan['decks'][i] for i in job['deck_indices']]
        for game_index, game in enumerate(result['games']):
            if game.get('truncated'):
                continue
            cand_seat = game['candidate_seat']
            ckpt = job.get('checkpoint'); opp_ckpt = job.get('opponent_checkpoint')
            seat_labels = ['', '']
            seat_labels[cand_seat] = f"CP{ckpt} ({mode_label(cand_mode)})"
            seat_labels[1 - cand_seat] = f"CP{opp_ckpt} ({mode_label(opp_mode)})"
            searches = {}
            for diag in game.get('search_diagnostics') or []:
                view = search_view(diag)
                if view is not None:
                    searches[str(diag['decision'])] = view
            game_id = f"{job['id']}-g{game_index}"
            detail = {
                'id': game_id,
                'seed': str(job['game_seed']),
                'deck1': decks[0]['cards'],
                'deck2': decks[1]['cards'],
                'actions': game['actions'],
                'seat_labels': seat_labels,
                'searches': searches,
            }
            (out_dir / 'games' / f'{game_id}.json').write_text(json.dumps(detail, separators=(',', ':')), encoding='utf-8')
            rows.append({
                'id': game_id,
                'block': job.get('block'),
                'archetypes': [decks[0]['name'], decks[1]['name']],
                'seat_labels': seat_labels,
                'first_player': game['first_player'],
                'winner': game.get('winner'),
                'terminal_reason': game.get('terminal_reason'),
                'moves': len(game['actions']),
                'searched': game.get('searched'),
            })
    index = {
        'title': title,
        'source': str(panel_dir),
        'model_sha256': plan.get('model_sha256'),
        'opponent_model_sha256': plan.get('opponent_model_sha256'),
        'note': 'Seats are 0-indexed here; the UI shows seat 0 as Player 1.',
        'games': rows,
    }
    (out_dir / 'index.json').write_text(json.dumps(index, indent=1), encoding='utf-8')
    return rows


def verify(out_dir: Path, rows: list[dict], count: int) -> int:
    """Replay games on the installed engine; record ``replayable`` per row.

    A recorded game can stop replaying when the engine's rules changed after
    it was played. Such rows stay in the index with ``replayable: false`` and
    a reason, so the watch page can skip them instead of failing mid-game.
    """
    from digimon_engine import RustHeadlessGame  # noqa: PLC0415 - optional dependency

    failures = 0
    for row in rows[:count]:
        detail = read(out_dir / 'games' / f"{row['id']}.json")
        game = RustHeadlessGame(detail['deck1'], detail['deck2'], False, False, False, int(detail['seed']))
        problem = None
        for step, action in enumerate(detail['actions']):
            if game.is_game_over:
                problem = f'game ended early at move {step}'
                break
            mask = game.get_action_mask()
            if action >= len(mask) or mask[action] != 1:
                problem = f'action {action} illegal at move {step}'
                break
            game.step(int(action))
        if problem is None and not game.is_game_over:
            problem = 'game not over after last recorded move'
        row['replayable'] = problem is None
        if problem:
            row['replay_problem'] = problem
            failures += 1
            print(f"DIVERGED {row['id']}: {problem}")
    index_path = out_dir / 'index.json'
    index = read(index_path)
    index['games'] = rows
    index['verified_games'] = min(count, len(rows))
    index_path.write_text(json.dumps(index, indent=1), encoding='utf-8')
    print(f'verified {min(count, len(rows))} games, {failures} diverged')
    return failures


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    parser.add_argument('--panel', required=True, type=Path)
    parser.add_argument('--name', required=True, help='queue folder name under public/watch/')
    parser.add_argument('--title', default=None)
    parser.add_argument('--out-root', type=Path, default=DEFAULT_OUT_ROOT)
    parser.add_argument('--verify', type=int, default=0, metavar='N')
    args = parser.parse_args()
    out_dir = args.out_root / args.name
    rows = build(args.panel, out_dir, args.title or args.name)
    print(f'wrote {len(rows)} games to {out_dir}')
    if args.verify:
        verify(out_dir, rows, args.verify)
    return 0


if __name__ == '__main__':
    sys.exit(main())
