"""Save one recorded decision from a frozen-evaluation panel as a search exam.

A search exam is a real game position that a model's search should handle in a
particular way ("give the Training line a real look", "don't slam Wingdramon
from hand"). The engine is deterministic from (decks, seed, actions), so the
exam stores those plus the searching seat's information regime and frozen
deck-list prior, and `examples/search_exam.rs` rebuilds and grades it.

    python code/tools/search_exam/make_exam.py --panel <panel dir> \
        --request match-015-1 --game 1 --decision 26 --id <exam-id> \
        --title "..." --expect visits:1002:0.10 --expect avoid:4 [--line name=1,2,3]

Expectations (plans are root action ids; multi-action plans join with '+'):
  visits:<plan>:<share>  the plan gets at least <share> of root visits
  avoid:<plan>           the plan is not the search's choice
  choose:<plan>          the plan is the search's choice
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
OUT_DIR = REPO / 'qa' / 'search-exams'


def read(path: Path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def plan_ids(text: str) -> list[int]:
    return [int(x) for x in text.split('+')]


def parse_expect(text: str) -> dict:
    kind, _, rest = text.partition(':')
    if kind == 'visits':
        plan, _, share = rest.rpartition(':')
        return {'kind': 'visit_share_at_least', 'plan': plan_ids(plan), 'share': float(share)}
    if kind == 'avoid':
        return {'kind': 'not_chosen', 'plan': plan_ids(rest)}
    if kind == 'choose':
        return {'kind': 'chosen', 'plan': plan_ids(rest)}
    raise SystemExit(f'unknown expectation {text!r}')


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    ap.add_argument('--panel', type=Path, required=True)
    ap.add_argument('--request', required=True)
    ap.add_argument('--game', type=int, required=True, help='index into the result games (0 or 1)')
    ap.add_argument('--decision', type=int, required=True, help='number of recorded actions before the position')
    ap.add_argument('--id', required=True)
    ap.add_argument('--title', required=True)
    ap.add_argument('--why', default='', help='what the search gets wrong here, in a sentence or two')
    ap.add_argument('--expect', action='append', default=[], required=True)
    ap.add_argument('--line', action='append', default=[], help='name=a,b,c line to score with the value head')
    ap.add_argument('--out-dir', type=Path, default=OUT_DIR)
    args = ap.parse_args()

    panel = args.panel
    plan = read(panel / 'plan.json')
    request = read(panel / 'requests' / f'{args.request}.json')
    result = read(panel / 'results' / f'{args.request}.json')
    game = result['games'][args.game]
    candidate_seat = game['candidate_seat']
    actions = game['actions']
    if args.decision >= len(actions):
        raise SystemExit('decision is past the end of the game')
    recorded = next((s for s in game['search_diagnostics'] if s['decision'] == args.decision), None)
    if recorded is None:
        raise SystemExit(f'decision {args.decision} was not searched (forced move)')
    viewer = recorded['diagnostics']['viewer']
    settings = request['candidate'] if viewer == candidate_seat else request['opponent']
    model_key = 'model' if viewer == candidate_seat else 'opponent_model'
    prior = read(panel / plan['prior'])
    decks = [plan['decks'][i]['cards'] for i in request['deck_indices']]
    labels = {tuple(p['plan']): p['action'].get('label') for p in recorded['opportunity']['plans']}

    def label(plan_list):
        return labels.get(tuple(plan_list))

    expectations = [parse_expect(e) for e in args.expect]
    for e in expectations:
        e['label'] = label(e['plan'])
    lines = {}
    for spec in args.line:
        name, _, acts = spec.partition('=')
        lines[name] = [int(a) for a in acts.split(',') if a]

    exam = {
        'schema': 'search-exam-v1',
        'id': args.id,
        'title': args.title,
        'why': args.why,
        'source': {
            'panel': str(panel),
            'request': args.request,
            'game_index': args.game,
            'decision': args.decision,
            'watch_queue_id': f"{args.request}-g{args.game}",
        },
        'setup': {
            'decks': decks,
            'game_seed': request['game_seed'],
            # Panel drivers set turn_count = 1 and enable belief knowledge
            # before the first action; the runner must do the same.
            'turn_count_reset': 1,
            'actions': actions[:args.decision],
        },
        'search': {
            'viewer': viewer,
            'regime': settings['information_regime'],
            'simulations': settings['simulations'],
            'worlds': settings['worlds'],
            'batch': settings['batch'],
            'search_seed': request['search_seed'],
            'seed_rule': 'search_seed + decision*0x9e3779b97f4a7c15 + viewer (wrapping)',
            'observation_profile': 'standard_lite_v2',
        },
        'model': {
            'name': Path(plan[model_key]).stem,
            'sha256': plan[f'{model_key}_sha256'],
        },
        # Frozen deck-list prior for the closed-list regime. The runner rebuilds
        # it with PriorSnapshot::new, which stamps the current build's legality
        # hash; the candidate lists are what matter.
        'prior': {k: prior[k] for k in ('format_id', 'population', 'candidates', 'unseen_mass', 'fallback_attempts')},
        'expectations': expectations,
        'lines': lines,
        'recorded': {
            'chosen': recorded['selected_plan'],
            'chosen_label': label(recorded['selected_plan']),
            'root_plans': [
                {'plan': p['plan'], 'label': p['action'].get('label'), 'visits': p['visits'],
                 'prior': p.get('prior_before_noise'), 'q': p.get('q')}
                for p in sorted(recorded['opportunity']['plans'], key=lambda p: -p['visits'])
            ],
        },
    }
    args.out_dir.mkdir(parents=True, exist_ok=True)
    out = args.out_dir / f'{args.id}.json'
    text = json.dumps(exam, indent=1) + '\n'
    out.write_text(text, encoding='utf-8')
    print(f'wrote {out} ({len(text) // 1024} KB, sha256 {hashlib.sha256(text.encode()).hexdigest()[:12]})')
    return 0


if __name__ == '__main__':
    sys.exit(main())
