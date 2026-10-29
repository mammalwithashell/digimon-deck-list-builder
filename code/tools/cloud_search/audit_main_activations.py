"""Inventory authored Main clauses in frozen training/evaluation snapshots.

This detects explicit OPT refunds, not every possible gameplay loop. Behavioral
tests are required for cancellation, costs, masks and cloned continuations.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path

import yaml

MAIN_TIMINGS = {'main', 'main_on_field', 'main_from_hand', 'main_from_trash'}


def walk(value, path='$'):
    if isinstance(value, dict):
        yield path, value
        for key, child in value.items():
            yield from walk(child, f'{path}.{key}')
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield from walk(child, f'{path}[{index}]')


def audit(card_ids, cards_dir):
    clauses, missing = [], []
    for card_id in sorted(set(card_ids)):
        path = cards_dir/card_id.split('-')[0].lower()/f'{card_id}.yaml'
        if not path.exists():
            path = cards_dir/'_examples'/f'{card_id}.yaml'
        if not path.exists():
            missing.append(card_id)
            continue
        card = yaml.safe_load(path.read_text(encoding='utf-8'))
        # Dual cards can put effects under faces rather than top-level effects.
        for location, clause in walk(card):
            timings = clause.get('when', [])
            timings = [timings] if isinstance(timings, str) else timings
            if not isinstance(timings, list) or not MAIN_TIMINGS.intersection(timings):
                continue
            refunds = [p for p, node in walk(clause.get('process', []), location+'.process') if 'refund_opt' in node]
            clauses.append(dict(card_id=card_id, name=card['name'], location=location,
                source=path.relative_to(cards_dir).as_posix(), when=timings,
                cap=1 if clause.get('once_per_turn') else clause.get('max_per_turn', 0),
                refund_paths=refunds, summary=clause.get('summary'),
                process_verbs=sorted({key for _, node in walk(clause.get('process', [])) for key in node})))
    return dict(card_ids=sorted(set(card_ids)), unique_cards=len(set(card_ids)),
        missing_sources=missing, main_card_count=len({c['card_id'] for c in clauses}),
        main_clause_count=len(clauses), clauses=clauses,
        refund_clause_count=sum(bool(c['refund_paths']) for c in clauses),
        limitation='Authored-clause scan; not whole-card fidelity or absence-of-all-loops certification')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--snapshot', type=Path, action='append', required=True)
    parser.add_argument('--cards-dir', type=Path, default=Path('code/digimon-engine/cards'))
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    ids, inputs = set(), []
    for path in args.snapshot:
        data = json.loads(path.read_text(encoding='utf-8'))
        decks = data if isinstance(data, list) else data['decks']
        contents = [deck.get('cards', deck.get('card_ids')) for deck in decks]
        if any(cards is None for cards in contents):
            raise ValueError('Snapshot lacks deck card IDs')
        for cards in contents:
            ids.update(cards)
        unique = {tuple(sorted(Counter(cards).items())) for cards in contents}
        inputs.append(dict(file=path.name, sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
                           slots=len(decks), distinct_lists=len(unique)))
    result = dict(inputs=inputs, **audit(ids, args.cards_dir))
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(result, indent=2)+'\n', encoding='utf-8')
    print(json.dumps({k:v for k,v in result.items() if k not in ('clauses','card_ids')}))
    return int(bool(result['missing_sources'] or result['refund_clause_count']))


if __name__ == '__main__':
    raise SystemExit(main())
