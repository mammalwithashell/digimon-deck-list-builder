"""Offline EX13 catalog payload generator. Defaults to validation/dry-run; no DCGO writes."""
import argparse
import hashlib
import json
from pathlib import Path

IDS = ('EX13-005','EX13-008','EX13-018','EX13-021','EX13-024','EX13-039','EX13-041','EX13-044','EX13-045')
COLORS = {name: i for i, name in enumerate(('Red','Blue','Yellow','Green','White','Black','Purple'))}
KINDS = {'Digimon': 0, 'Digi-Egg': 3}
RARITIES = {name: i for i, name in enumerate(('C','U','R','SR','UR','SEC','P','None'))}

def allocate_indices(existing, requested):
    values = list(existing.values())
    if len(values) != len(set(values)) or any(type(x) is not int or x < 0 for x in values):
        raise ValueError('Invalid or duplicate existing index')
    next_index = max(values, default=-1) + 1
    result = {}
    for cid in sorted(set(requested)):
        if cid in existing: result[cid] = existing[cid]
        else:
            if next_index >= 65536: raise ValueError('Deck-code index capacity exhausted')
            result[cid] = next_index
            next_index += 1
    return result

def map_card(raw):
    if raw.get('layout') != 'single' or raw.get('printing_field_gaps'):
        raise ValueError('Unsupported or incomplete official layout')
    if raw['card_id'] not in IDS or not raw.get('name') or raw.get('card_type') not in KINDS:
        raise ValueError('Unsupported identity/kind')
    if not raw.get('colors') or any(c not in COLORS for c in raw['colors']):
        raise ValueError('Unsupported colors')
    if raw.get('grade_evolution_routes') or raw.get('tamer_evolution_routes'):
        raise ValueError('Unsupported evolution route')
    def words(value): return [s.strip() for s in value.split('/') if s.strip()]
    main, inherited, security = [], [], []
    for section in raw['text_sections']:
        label, text = section['label'], section['text']
        if label in ('Inherited Effect', 'Inheritable Effect'): inherited.append(text)
        elif label == 'Security Effect': security.append(text)
        elif label in ('Effect','Special Digivolution Condition','Special Play Condition','DNA Digivolution','Rule'): main.append(text)
        else: raise ValueError('Unsupported text section ' + label)
    # No ACE/Link/dual metadata silently defaulting for a future expansion of this pool.
    if any('Overflow' in s or 'Link' in s for s in main): raise ValueError('Unsupported ACE/Link')
    costs = []
    for route in raw['evolution_routes']:
        for color in route['colors']:
            costs.append(dict(CardColor=COLORS[color], Level=route['from_level'], MemoryCost=route['cost']))
    egg = raw['card_type'] == 'Digi-Egg'
    if not egg and (type(raw['play_cost']) is not int or type(raw['dp']) is not int):
        raise ValueError('Missing Digimon cost/DP')
    return dict(CardID=raw['card_id'], CardSpriteName=raw['card_id'], CardEffectClassName=raw['card_id'].replace('-','_'),
                CardName_ENG=raw['name'], cardColors=[COLORS[c] for c in raw['colors']], cardKind=[KINDS[raw['card_type']]],
                PlayCost=-1 if egg else raw['play_cost'], DP=0 if egg else raw['dp'], Level=raw['level'],
                Form_ENG=words(raw['form']), Attribute_ENG=words(raw['attribute'])+raw['rule_grants']['attributes'],
                Type_ENG=words(raw['type'])+raw['rule_grants']['traits'], EvoCosts=costs, rarity=RARITIES[raw['rarity']],
                EffectDiscription_ENG='\n\n'.join(main), InheritedEffectDiscription_ENG='\n\n'.join(inherited),
                SecurityEffectDiscription_ENG='\n\n'.join(security), MaxCountInDeck=4, OverflowMemory=0,
                LinkDP=0, LinkEffect='', LinkRequirement='', dualEffect='', OptionCardColorRequirements=[], OptionEffect='')

def load_evidence(directory):
    directory = Path(directory)
    try: manifest = json.loads((directory/'manifest.json').read_text(encoding='utf-8'))
    except FileNotFoundError as exc: raise ValueError('Missing official evidence manifest') from exc
    if manifest.get('schema_version') != 1 or not manifest.get('image_reviewed') or manifest.get('applicable_corrections') != []:
        raise ValueError('Evidence needs image review or correction adjudication')
    records = manifest['records']
    if sorted(r['card_id'] for r in records) != list(IDS): raise ValueError('Evidence must contain exact nine-card pool')
    rows = []
    for record in records:
        cid = record['card_id']
        if record['source_url'] != 'https://world.digimoncard.com/cards/?search=true&card_no='+cid:
            raise ValueError('Unexpected authority URL')
        for name in (cid+'.html',cid+'.png',cid+'.json'):
            if hashlib.sha256((directory/name).read_bytes()).hexdigest() != record['hashes'][name]:
                raise ValueError('Evidence hash mismatch: '+name)
        raw = json.loads((directory/(cid+'.json')).read_text(encoding='utf-8'))
        if raw['card_id'] != cid: raise ValueError('Evidence identity mismatch')
        rows.append(map_card(raw))
    return rows

def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--evidence', type=Path, default=Path(__file__).with_name('evidence'))
    ap.add_argument('--write', type=Path, help='Explicit NEW payload file; default prints dry-run JSON')
    args = ap.parse_args(argv)
    payload = dict(schemaVersion=1, cards=load_evidence(args.evidence),
                   evidenceManifestSha256=hashlib.sha256((args.evidence/'manifest.json').read_bytes()).hexdigest())
    text = json.dumps(payload, ensure_ascii=False, indent=2)+'\n'
    if args.write:
        with args.write.open('x', encoding='utf-8') as output: output.write(text)
    else: print(text, end='')
    return 0

if __name__ == '__main__': raise SystemExit(main())
