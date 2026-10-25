"""Freeze the admitted local Light Hybrid list into a new 13-archetype pool.

Retains all old lists and their ordering; two sampling slots for the one pet
list preserve equal archetype weight, without claiming two distinct variants.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import shutil


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--inputs', type=Path, required=True)
    parser.add_argument('--deck', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    import digimon_engine
    from digimon_gym.agents.gauntlet import (DeckEntry, GeneralistDeckPool,
        require_mechanics_ready, stable_deck_id, validate_admissible_deck)
    from model_contract import current_contract
    require_mechanics_ready()
    raw = args.deck.read_bytes()
    cards = []
    for line in raw.decode('utf-8-sig').splitlines():
        match = re.fullmatch(r'\s*(\d+)\s+.+?\s+([A-Z0-9]+-\d+)\s*', line)
        if match:
            cards.extend([match[2]] * int(match[1]))
    assert len(cards) == 54 and len(set(cards)) == 20, 'Unexpected exact Light Hybrid list'
    assert hashlib.sha256(raw).hexdigest() == '92ec23d1c154591f50c0ced36ffbede106f9bb84d7ef01d810e5485b22994091'
    validate_admissible_deck(cards, label='Light Hybrid local DCGO')
    registered = set(digimon_engine.load_implemented_card_ids())
    assert set(cards) <= registered
    assert digimon_engine.validate_deck(cards).is_valid
    train = GeneralistDeckPool.from_snapshot(args.inputs / 'train_snapshot.json', implemented_card_ids=registered)
    pets = GeneralistDeckPool.from_snapshot(args.inputs / 'pet_evaluation_snapshot.json', implemented_card_ids=registered)
    assert 'Light Hybrid' not in train.archetypes
    entry = DeckEntry(deck_id=stable_deck_id(cards), archetype_name='Light Hybrid', card_ids=cards,
                      source='local_dcgo', source_deck_id=args.deck.stem)
    train.archetypes['Light Hybrid'] = [entry]
    pets.archetypes['Light Hybrid'] = [entry]
    slots = json.loads((args.inputs / 'decks.json').read_text())
    for row in slots:
        validate_admissible_deck(row['cards'], label=row['name'])
        assert set(row['cards']) <= registered
        assert digimon_engine.validate_deck(row['cards']).is_valid
    slots += [dict(name=f'Light Hybrid:{entry.deck_id}:slot{i}', cards=cards) for i in range(2)]
    counts = Counter(row['name'].split(':sha256:', 1)[0] for row in slots)
    assert len(counts) == 13 and set(counts.values()) == {2}
    report = json.loads((args.inputs / 'pool_report.json').read_text())
    admission = dict(archetype='Light Hybrid', source=str(args.deck), source_sha256=hashlib.sha256(raw).hexdigest(),
                     cards=cards, unique_ids=20, copies=54, admitted=True, reasons=[],
                     contract=current_contract('standard_lite_v2'))
    report['pet_admission'] = [r for r in report['pet_admission'] if r['archetype'] != 'Light Hybrid'] + [admission]
    report.update(scope='Eight frozen English EX13 meta archetypes plus five eligible exact local DCGO pet lists',
                  archetypes=sorted(counts), distinct_training_lists=train.deck_count, native_sampling_slots=len(slots),
                  games_per_generation=1100, expected_appearances_per_archetype=2200 / 13,
                  light_hybrid_scope='One exact list repeated in two slots; same-list evaluation, not held-out generalization')
    args.out.mkdir(parents=True, exist_ok=False)
    train.write_snapshot(args.out / 'train_snapshot.json')
    pets.write_snapshot(args.out / 'pet_evaluation_snapshot.json')
    shutil.copy2(args.inputs / 'heldout_snapshot.json', args.out / 'heldout_snapshot.json')
    (args.out / 'LightHybrid.txt').write_bytes(raw)
    for name, value in [('decks.json', slots), ('pool_report.json', report), ('light-hybrid-admission.json', admission)]:
        (args.out / name).write_text(json.dumps(value, indent=2) + '\n')
    print(json.dumps(dict(archetypes=len(counts), distinct_lists=train.deck_count, slots=len(slots),
                         light_hybrid=admission, games_per_generation=1100)))


if __name__ == '__main__':
    main()
